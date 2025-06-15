use crate::database::DatabasePool;
use crate::database::accounts::Account;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::Message;

// all distances are in kilometers
const TALK_RADIUS: f64 = 10.0;
const EARTH_RADIUS: f64 = 6371.0;

pub struct GlobalData {
    pub connections: RwLock<HashMap<u32, Arc<ConnectionData>>>,
    pub database_pool: DatabasePool,
}

pub struct ConnectionData {
    /// Unique identifier for this connection
    pub id: u32,

    /// The account associated with this connection, set after login
    pub account: RwLock<Account>,

    /// The current gps location
    pub position: RwLock<Position>,

    /// The selected frequency
    pub frequency: RwLock<u8>,

    /// The current room id, if any
    pub room: RwLock<Option<u32>>,

    /// A tokio mpsc channel for sending messages to this connection
    pub channel: Sender<Message>,

    /// Cached list of connection ids that are able to hear this connection, reevaluated periodically
    pub broadcast: RwLock<Vec<u32>>,
}

impl ConnectionData {
    pub async fn reevaluate_broadcast(&self, list: &Vec<Arc<ConnectionData>>) {
        let room = async { self.room.read().await.clone() }.await;

        let mut results = Vec::new();
        if let Some(room) = room {
            for conn in list {
                if conn.id == self.id {
                    continue;
                }

                let other_room = conn.room.read().await.clone();
                if other_room != Some(room) {
                    continue;
                }

                results.push(conn.id);
            }
        } else {
            let frequency = self.frequency.read().await.clone();
            let position = self.position.read().await.clone();

            for conn in list {
                if conn.id == self.id {
                    continue;
                }

                let other_frequency = conn.frequency.read().await.clone();
                if other_frequency != frequency {
                    continue;
                }

                let other_position = conn.position.read().await.clone();
                if position.distance(&other_position) <= TALK_RADIUS {
                    results.push(conn.id);
                }
            }
        }

        println!(
            "Reevaluated broadcast for connection {}: {:?}",
            self.id, results
        );

        let mut broadcast = self.broadcast.write().await;
        *broadcast = results;
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

impl Position {
    /// Calculates the distance to another position using the Haversine formula
    /// More accurate than pythagoras because, yes indeed, earth is not flat
    pub fn distance(&self, other: &Position) -> f64 {
        let lat_diff = self.latitude - other.latitude;
        let lon_diff = self.longitude - other.longitude;

        let lat_diff = lat_diff.to_radians();
        let lon_diff = lon_diff.to_radians();

        let a = (lat_diff / 2.0).sin().powi(2)
            + self.latitude.to_radians().cos()
                * other.latitude.to_radians().cos()
                * (lon_diff / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS * c
    }
}
