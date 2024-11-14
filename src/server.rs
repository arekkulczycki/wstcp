use crate::channel::ProxyChannel;
use crate::{Error, Result};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::net::TcpListener;


/// WebSocket to TCP proxy server.
#[derive(Debug)]
pub struct ProxyServer {
    real_server_addr: SocketAddr,
    incoming: TcpListener,
}
impl ProxyServer {
    /// Makes a new `ProxyServer` instance.
    pub async fn new(
        incoming: TcpListener,
        real_server_addr: SocketAddr,
    ) -> Result<ProxyServer> {
        log::info!("Starts a WebSocket proxy server");
        Ok(ProxyServer {
            real_server_addr,
            incoming,
        })
    }
}
impl Future for ProxyServer {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.incoming).poll_accept(cx) {
                Poll::Pending => {
                    break;
                }
                Poll::Ready(Ok((stream, addr))) => {
                    log::debug!("New client arrived: {:?}", addr);

                    let channel = ProxyChannel::new(stream, this.real_server_addr);
                    tokio::spawn(async move {
                        match channel.await {
                            Err(e) => {
                                log::warn!("A proxy channel aborted: {}", e);
                            }
                            Ok(()) => {
                                log::info!("A proxy channel terminated normally");
                            }
                        }
                    });
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(track!(Error::from(e))));
                }
            }
        }
        Poll::Pending
    }
}
