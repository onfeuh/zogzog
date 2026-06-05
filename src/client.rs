use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::{
    io,
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc,
    task::JoinSet,
};

use crate::w3gs::is_w3gs;
use crate::{NetworkEvent, WC3_GAME_PORT};

const WC3C_CLIENT_PING: [u8; 4] = [0xf7, 0x00, 0x04, 0x00];

#[cfg(target_os = "windows")]
const PLATFORM_WINDOWS_WSAECONNRESET: i32 = 10054;

pub async fn main(
    bridge_addr: SocketAddr,
    proxy_addr: SocketAddr,
    tx: mpsc::Sender<NetworkEvent>,
) -> io::Result<()> {
    let mut set: JoinSet<io::Result<()>> = JoinSet::new();

    tx.send(NetworkEvent::Connecting).await.ok();

    let bridge_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let local_socket = Arc::new(UdpSocket::bind((proxy_addr.ip(), 0)).await?);
    let local_instance = SocketAddr::from((Ipv4Addr::LOCALHOST, WC3_GAME_PORT));

    bridge_socket.connect(bridge_addr).await?;

    // send WC3C_CLIENT_PING to remote host every 4 seconds
    set.spawn({
        let bridge_socket = bridge_socket.clone();
        let tx = tx.clone();
        async move {
            loop {
                bridge_socket.send(&WC3C_CLIENT_PING).await?;
                tx.send(NetworkEvent::PacketSent {
                    addr: bridge_addr,
                    data: WC3C_CLIENT_PING.to_vec(),
                })
                .await
                .ok();
                sleep(Duration::from_secs(4)).await;
            }
        }
    });

    // TCP proxy
    set.spawn(async move {
        let listener = TcpListener::bind((proxy_addr.ip(), WC3_GAME_PORT)).await?;
        loop {
            let (client, _) = listener.accept().await?;
            tokio::spawn(async move {
                let host = TcpStream::connect(bridge_addr).await.unwrap();
                let (mut cr, mut cw) = client.into_split();
                let (mut hr, mut hw) = host.into_split();
                let _ = tokio::join!(
                    tokio::io::copy(&mut cr, &mut hw),
                    tokio::io::copy(&mut hr, &mut cw),
                );
            });
        }
    });

    // bridge to game
    set.spawn({
        let bridge_socket = bridge_socket.clone();
        let local_socket = local_socket.clone();
        let tx = tx.clone();
        async move {
            let mut buf = [0u8; 1024];
            loop {
                match bridge_socket.recv_from(&mut buf).await {
                    Ok((len, src)) => {
                        let data = &buf[..len];

                        if !is_w3gs(data) {
                            // skip non warcraft 3 game server packet
                            continue;
                        }

                        tx.send(NetworkEvent::PacketReceived {
                            addr: src,
                            data: data.to_vec(),
                        })
                        .await
                        .ok();
                        local_socket.send_to(data, local_instance).await?;
                        tx.send(NetworkEvent::PacketSent {
                            addr: local_instance,
                            data: data.to_vec(),
                        })
                        .await
                        .ok();
                    }
                    #[cfg(target_os = "windows")]
                    Err(e) if e.raw_os_error() == Some(PLATFORM_WINDOWS_WSAECONNRESET) => {
                        continue;
                    }
                    Err(e) => break Err(e),
                }
            }
        }
    });

    tx.send(NetworkEvent::Connected).await.ok();

    if let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(())) => {
                panic!("task finished prematurily")
            }
            Ok(Err(e)) => {
                set.abort_all();
                return Err(e);
            }
            Err(join_err) => {
                // task panicked
                set.abort_all();
                return Err(io::Error::other(format!("task join: {join_err}")));
            }
        }
    }

    Ok(())
}
