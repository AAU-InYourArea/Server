#![feature(fn_traits)]

pub mod database;
pub mod hash;
pub mod endpoints;
pub mod data;
pub mod error;
pub mod messages;

use std::collections::HashMap;
use std::env;
use std::io::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::FusedStream;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::MySqlPool;
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::WebSocketStream;
use crate::data::{ConnectionData, GlobalData, Position};
use crate::database::accounts::{create_account, get_by_username, set_session};
use crate::database::rooms::delete_room;
use crate::endpoints::direct_request;
use crate::error::{AnyErr, ProtocolError};
use crate::hash::{hash, random_session, verify};
use crate::messages::login::{LoginRequest, LoginResponse};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // read database configuration from environment variables
    let database_pool = Arc::new(
        MySqlPool::connect_with(MySqlConnectOptions::new()
            .host(env::var("DB_HOST").expect("DB_HOST not set").as_str())
            .port(env::var("DB_PORT").expect("DB_PORT not set").parse().expect("DB_PORT not parseable"))
            .database(env::var("DB_NAME").expect("DB_NAME not set").as_str())
            .username(env::var("DB_USER").expect("DB_USER not set").as_str())
            .password(env::var("DB_PASS").expect("DB_PASS not set").as_str())
        ).await.expect("Failed to connect to database")
    );

    // read WebSocket address from environment variables
    // bind socket to this address
    let listen_addr = env::var("WS_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listen_addr = || listen_addr.clone();
    let listener = TcpListener::bind(listen_addr()).await.expect(format!("Failed to bind to {}", listen_addr()).as_str());
    println!("Listening on {}", listen_addr());

    // create global data struct to hold connections and database pool
    // wrapped in Arc (atomic reference) for shared ownership
    let global_data = GlobalData {
        connections: RwLock::new(HashMap::new()),
        database_pool,
    };
    let global_data = Arc::new(global_data);
    
    // every connection gets its own id for easier linking to each other
    let mut id = 0;
    while let Ok((stream, addr)) = listener.accept().await {
        id += 1;
        let global_data = global_data.clone();
        tokio::spawn(async move { // each connection is handled in its own task
            if let Err(err) = handle_connection(global_data, stream, addr, id).await {
                println!("Connection from {} closed: {}", addr, err)
            } else {
                println!("Connection from {} closed", addr)
            }
        });
    }

    println!("Server stopped listening for connections.");

    Ok(())
}

async fn handle_connection(global_data: Arc<GlobalData>, stream: TcpStream, addr: SocketAddr, id: u32) -> Result<(), AnyErr> {
    // first upgrade the raw TCP stream to a WebSocket stream
    // this does all the necessary handshakes and protocol upgrades internally
    let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;

    println!("New WebSocket connection from {}", addr);

    let database_pool = global_data.database_pool.clone();
    let mut account;
    { // first wait for a login/register request
        let mut tries = 0;
        loop {
            let msg: LoginRequest = expect_protocol(&mut ws_stream, ProtocolError::LoginRequired).await?;
            if msg.register {
                // if the request is a register request, we try to create a new account
                account = create_account(&database_pool, &msg.username, &hash(&msg.password)?).await;
                if account.is_ok() {
                    break;
                }
            } else {
                // if the request is a login request, we try to get the account by username
                account = get_by_username(&database_pool, &msg.username).await;
                if let Ok(acc) = &account {
                    // if the account exists, we check the session token or password to allow login
                    if msg.session {
                        if let Some(session) = &acc.session {
                            if msg.password.eq(session) {
                                break;
                            }
                        }
                    } else if verify(&msg.password, &acc.password_hash)? {
                        break;
                    }
                }
            }

            // if we reach this point, the login failed
            // we send a response indicating login failure
            send_protocol(&mut ws_stream, LoginResponse {
                success: false,
                username: None,
                session: None
            }).await?;

            // only allow 3 tries before disconnecting
            tries += 1;
            if tries >= 3 {
                return Err(ProtocolError::InvalidCredentials.into());
            }
        }
    }
    let mut account = account?;

    // generate a new session token for the account
    let session = random_session();
    set_session(&database_pool, account.id, Some(session.clone())).await?;
    account.session = Some(session.clone());

    // send a response indicating login success
    send_protocol(&mut ws_stream, LoginResponse {
        success: true,
        username: Some(account.username.clone()),
        session: Some(session)
    }).await?;

    // create a new ConnectionData struct to hold required data
    let account = RwLock::new(account);
    let (send, mut recv) = channel(32); // channel for sending messages to the WebSocket stream from other tasks
    let data = ConnectionData {
        id,
        account,
        position: RwLock::new(Position {
            latitude: 0.0,
            longitude: 0.0,
        }),
        frequency: RwLock::new(0),
        room: RwLock::new(None),
        channel: send,
        broadcast: RwLock::new(vec![]),
    };
    let data = Arc::new(data);

    { // write-lock and insert the new connection data into the global data
        let mut connections = global_data.connections.write().await;
        connections.insert(id, data.clone());
    }

    let mut ticker = interval(Duration::from_secs(1));
    loop {
        if ws_stream.is_terminated() {
            break;
        }

        // use select! to handle multiple futures concurrently
        select! {
            _ = ticker.tick() => { // this will run every second
                // periodically reevaluate the broadcast list (who talks to whom)
                let connections = global_data.connections.read().await;
                data.reevaluate_broadcast(&connections.values().cloned().collect()).await;
            }
            recv_msg = ws_stream.next() => { // this will receive messages from the WebSocket stream
                if let Some(Ok(msg)) = recv_msg {
                    // call direct_request to process the message
                    if let Err(err) = direct_request(global_data.clone(), data.clone(), msg).await {
                        eprintln!("Error processing message: {}", err);
                        break;
                    }
                }
            }
            send_msg = recv.recv() => { // this will receive messages from the channel in ConnectionData
                if let Some(msg) = send_msg {
                    // we simply passthrough the message to the WebSocket stream
                    if let Err(err) = ws_stream.send(msg).await {
                        eprintln!("Error sending message: {}", err);
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    { // write-lock and remove the connection data from the global data
        let mut connections = global_data.connections.write().await;
        connections.remove(&id);
    }
    { // read-lock and check if the room is empty
        let room = data.room.read().await;
        if let Some(room_id) = *room {
            check_chatroom_empty(&global_data, room_id).await?;
        }
    }

    Ok(())
}

/// Expects a protocol message from the WebSocket stream.
/// If the message is not a text message, it will return an error.
/// If the message can not be deserialized as the expected type, it will return an error.
async fn expect_protocol<T: DeserializeOwned>(ws_stream: &mut WebSocketStream<TcpStream>, error: ProtocolError) -> Result<T, AnyErr> {
    loop {
        let msg = ws_stream.next().await.ok_or(error)??;

        if msg.is_text() {
            let msg = msg.into_text()?;
            let parsed = serde_json::from_str(&msg)?;
            return Ok(parsed);
        } else if msg.is_binary() || msg.is_close() {
            return Err(error.into())
        }
    }
}

/// converts a message to json and sends it as a text message over the WebSocket stream.
async fn send_protocol<T: Serialize>(ws_stream: &mut WebSocketStream<TcpStream>, msg: T) -> Result<(), AnyErr> {
    let msg = serde_json::to_string(&msg)?;
    ws_stream.send(Message::Text(Utf8Bytes::from(msg))).await?;
    Ok(())
}

/// Checks if the chatroom with the given room_id is empty.
/// If it is empty, it deletes the room from the database.
pub async fn check_chatroom_empty(global_data: &GlobalData, room_id: i32) -> Result<bool, AnyErr> {
    let connections = global_data.connections.read().await;
    for connection in connections.values() { // check if any connection is still in the room
        let connection_room = connection.room.read().await;
        if let Some(room) = *connection_room {
            if room == room_id {
                return Ok(false);
            }
        }
    }

    // if we reach this point, the room is empty
    // delete the room from the database
    delete_room(&global_data.database_pool, room_id).await?;
    Ok(true)
}