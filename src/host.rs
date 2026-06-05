use std::collections::HashMap;
use std::time::Instant;
use std::{sync::Arc, time::Duration};

use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio::{io, net::UdpSocket, sync::mpsc, task::JoinSet, time::sleep};

use crate::w3gs::{is_greeting, is_w3gs};
use crate::{
    NetworkEvent, WC3_GAME_PORT,
    w3gs::{GameType, searchgame_payload},
};

const PEER_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn main(
    bridge_addr: SocketAddr,
    game_type: GameType,
    version: u32,
    tx: mpsc::Sender<NetworkEvent>,
) -> io::Result<()> {
    let mut set: JoinSet<io::Result<()>> = JoinSet::new();

    let socket = Arc::new(UdpSocket::bind(bridge_addr).await?);
    let local_instance = SocketAddr::from((Ipv4Addr::LOCALHOST, WC3_GAME_PORT));

    // send SEARCHGAME to local WC3 every 2s
    set.spawn({
        let socket = socket.clone();
        let tx = tx.clone();
        async move {
            let payload: [u8; 16] = searchgame_payload(game_type, version);
            let socket = socket.clone();
            let tx = tx.clone();
            loop {
                socket.send_to(&payload, &local_instance).await?;
                tx.send(NetworkEvent::PacketSent {
                    addr: local_instance,
                    data: payload.to_vec(),
                })
                .await
                .ok();
                sleep(Duration::from_secs(2)).await;
            }
        }
    });

    // TCP proxy
    set.spawn(async move {
        let listener = TcpListener::bind(("0.0.0.0", bridge_addr.port())).await?;
        loop {
            let (incoming, _) = listener.accept().await?;
            tokio::spawn(async move {
                let wc3 = TcpStream::connect(("127.0.0.1", WC3_GAME_PORT))
                    .await
                    .unwrap();
                let (mut ir, mut iw) = incoming.into_split();
                let (mut wr, mut ww) = wc3.into_split();
                let _ = tokio::join!(
                    tokio::io::copy(&mut ir, &mut ww),
                    tokio::io::copy(&mut wr, &mut iw),
                );
            });
        }
    });

    // broadcast to peers
    set.spawn({
        let socket = socket.clone();
        let tx = tx.clone();
        async move {
            let mut buf = [0u8; 1024];
            let mut peers: HashMap<SocketAddr, Instant> = HashMap::new();
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, sender)) => {
                        let data = &buf[..len];

                        if !is_w3gs(data) {
                            // skip non warcraft 3 game server packet
                            continue;
                        }

                        tx.send(NetworkEvent::PacketReceived {
                            addr: sender,
                            data: data.to_vec(),
                        })
                        .await
                        .ok();

                        let now = Instant::now();

                        // drop stale peers
                        peers.retain(|_, &mut last_seen| {
                            now.duration_since(last_seen) < PEER_TIMEOUT
                        });

                        let count = peers.len();

                        tx.send(NetworkEvent::PeerCount(count)).await.ok();

                        if is_greeting(data) {
                            peers.insert(sender, Instant::now());
                            continue;
                        }

                        // broadcast to survivors except sender
                        for (p, _) in peers.iter().filter(|(p, _)| **p != sender) {
                            socket.send_to(data, p).await?;
                            tx.send(NetworkEvent::PacketSent {
                                addr: *p,
                                data: data.to_vec(),
                            })
                            .await
                            .ok();
                        }
                    }
                    Err(e) => break Err::<(), io::Error>(e),
                }
            }
        }
    });

    tx.send(NetworkEvent::ServerStarted).await.ok();

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
