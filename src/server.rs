use crate::channel::ProxyChannel;
use crate::{Error, Result};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

#[cfg(feature = "tokio")]
use tokio::net::TcpListener;

#[cfg(feature = "async-std")]
use async_std::{net::Incoming, stream::Stream};

/// WebSocket to TCP proxy server.
#[derive(Debug)]
#[cfg(feature = "tokio")]
pub struct ProxyServer {
    real_server_addr: SocketAddr,
    incoming: TcpListener,
}


/// WebSocket to TCP proxy server.

#[derive(Debug)]
#[cfg(feature = "async-std")]
pub struct ProxyServer<'a> {
    real_server_addr: SocketAddr,
    incoming: Incoming<'a>,
}

#[cfg(feature = "tokio")]
impl ProxyServer {
    /// Makes a new `ProxyServer` instance.
    pub async fn new(incoming: TcpListener, real_server_addr: SocketAddr) -> Result<ProxyServer> {
        log::info!("Starts a WebSocket proxy server");
        Ok(ProxyServer {
            real_server_addr,
            incoming,
        })
    }
}

#[cfg(feature = "async-std")]
impl<'a> ProxyServer<'a> {
    /// Makes a new `ProxyServer` instance.
    pub async fn new(incoming: Incoming<'a>, real_server_addr: SocketAddr) -> Result<ProxyServer<'a>> {
        log::info!("Starts a WebSocket proxy server");
        Ok(ProxyServer {
            real_server_addr,
            incoming,
        })
    }
}

#[cfg(feature = "tokio")]
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

#[cfg(feature = "async-std")]
impl<'a> Future for ProxyServer<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.incoming).poll_next(cx) {
                Poll::Pending => {
                    break;
                }
                Poll::Ready(None) => {
                    log::warn!("TCP socket for the WebSocket proxy server has been closed");
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(track!(Error::from(e))));
                }
                Poll::Ready(Some(Ok(stream))) => {
                    let addr = stream.peer_addr()?;
                    log::debug!("New client arrived: {:?}", addr);

                    let channel = ProxyChannel::new(stream, this.real_server_addr);
                    async_std::task::spawn(async move {
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
            }
        }
        Poll::Pending
    }
}