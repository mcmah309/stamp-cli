use axum::{
    extract::{
        ws::{self, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::StreamExt;

pub async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket))
}

pub async fn handle_connection(socket: WebSocket) {
    let (socket_tx, mut socket_rx) = socket.split();

    while let Some(socket_message) = socket_rx.next().await {
        match socket_message {
            Ok(ws::Message::Text(text)) => {
                // todo
            }
            Ok(ws::Message::Binary(_)) => {
                // todo
            }
            Ok(ws::Message::Ping(_)) | Ok(ws::Message::Pong(_)) => {
                // heartbeat: no need to handle
            }
            Ok(ws::Message::Close(_)) | Err(_) => {
                tracing::info!("WebSocket connection closed");
                // todo
                break;
            }
        }
    }
}