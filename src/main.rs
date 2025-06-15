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
use crate::endpoints::direct_request;
use crate::error::{AnyErr, ProtocolError};
use crate::hash::{hash, random_session, verify};
use crate::messages::login::{LoginRequest, LoginResponse};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database_pool = Arc::new(
        MySqlPool::connect_with(MySqlConnectOptions::new()
            .host(env::var("DB_HOST").expect("DB_HOST not set").as_str())
            .port(env::var("DB_PORT").expect("DB_PORT not set").parse().expect("DB_PORT not parseable"))
            .database(env::var("DB_NAME").expect("DB_NAME not set").as_str())
            .username(env::var("DB_USER").expect("DB_USER not set").as_str())
            .password(env::var("DB_PASS").expect("DB_PASS not set").as_str())
        ).await.expect("Failed to connect to database")
    );

    let listen_addr = env::var("WS_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listen_addr = || listen_addr.clone();
    let listener = TcpListener::bind(listen_addr()).await.expect(format!("Failed to bind to {}", listen_addr()).as_str());
    println!("Listening on {}", listen_addr());

    let global_data = GlobalData {
        connections: RwLock::new(HashMap::new()),
        database_pool,
    };
    let global_data = Arc::new(global_data);
    
    let mut id = 0;
    while let Ok((stream, addr)) = listener.accept().await {
        id += 1;
        let global_data = global_data.clone();
        tokio::spawn(async move {
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
    let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;

    println!("New WebSocket connection from {}", addr);

    let database_pool = global_data.database_pool.clone();
    let mut account;
    {
        let mut tries = 0;
        loop {
            let msg: LoginRequest = expect_protocol(&mut ws_stream, ProtocolError::LoginRequired).await?;
            if msg.register {
                account = create_account(&database_pool, &msg.username, &hash(&msg.password)?).await;
                if account.is_ok() {
                    break;
                }
            } else {
                account = get_by_username(&database_pool, &msg.username).await;
                if let Ok(acc) = &account {
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

            send_protocol(&mut ws_stream, LoginResponse {
                success: false,
                username: None,
                session: None
            }).await?;

            tries += 1;
            if tries >= 3 {
                return Err(ProtocolError::InvalidCredentials.into());
            }
        }
    }
    let mut account = account?;

    let session = random_session();
    set_session(&database_pool, account.id, Some(session.clone())).await?;
    account.session = Some(session.clone());

    send_protocol(&mut ws_stream, LoginResponse {
        success: true,
        username: Some(account.username.clone()),
        session: Some(session)
    }).await?;

    let account = RwLock::new(account);
    let (send, mut recv) = channel(32);
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

    {
        let mut connections = global_data.connections.write().await;
        connections.insert(id, data.clone());
    }

    let mut ticker = interval(Duration::from_secs(1));
    loop {
        if ws_stream.is_terminated() {
            break;
        }

        select! {
            _ = ticker.tick() => {
                let connections = global_data.connections.read().await;
                data.reevaluate_broadcast(&connections.values().cloned().collect()).await;
            }
            recv_msg = ws_stream.next() => {
                if let Some(Ok(msg)) = recv_msg {
                    direct_request(global_data.clone(), data.clone(), msg).await?;
                }
            }
            send_msg = recv.recv() => {
                if let Some(msg) = send_msg {
                    ws_stream.send(msg).await?;
                } else {
                    break;
                }
            }
        }
    }

    {
        let mut connections = global_data.connections.write().await;
        connections.remove(&id);
    }

    Ok(())
}

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

async fn send_protocol<T: Serialize>(ws_stream: &mut WebSocketStream<TcpStream>, msg: T) -> Result<(), AnyErr> {
    let msg = serde_json::to_string(&msg)?;
    ws_stream.send(Message::Text(Utf8Bytes::from(msg))).await?;
    Ok(())
}