use crate::channel::ProxyChannel;
use crate::{Error, Result};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;


/// WebSocket to TCP proxy server.
#[derive(Debug)]
pub struct ProxyServer {
    real_server_addr: SocketAddr,
    incoming: TcpListener,
    cancellation_token: CancellationToken
}
impl ProxyServer {
    /// Makes a new `ProxyServer` instance.
    pub async fn new(
        incoming: TcpListener,
        real_server_addr: SocketAddr,
        cancellation_token: CancellationToken,
    ) -> Result<ProxyServer> {
        log::info!("Starts a WebSocket proxy server");
        Ok(ProxyServer {
            real_server_addr,
            incoming,
            cancellation_token,
        })
    }
}
impl Future for ProxyServer {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            if this.cancellation_token.is_cancelled() {
                return Poll::Ready(Ok(()));
            }

            match Pin::new(&mut this.incoming).poll_accept(cx) {
                Poll::Pending => {
                    log::debug!("server pending...");
                    break;
                }
                Poll::Ready(Ok((stream, addr))) => {
                    log::debug!("New client arrived: {:?}", addr);

                    let child_token = this.cancellation_token.child_token();
                    let channel = ProxyChannel::new(stream, this.real_server_addr, child_token);
                    tokio::spawn(async move {
                        let result = channel.await;

                        match result {
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
                    log::debug!("server ready...");
                    return Poll::Ready(Err(track!(Error::from(e))));
                }
            }
        }
        Poll::Pending
    }
}
