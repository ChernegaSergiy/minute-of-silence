use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use khvylyna_pairing::{transport, window::PairingWindow};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn pairing_code_accepts_only_one_encrypted_transfer() {
    let window = Arc::new(Mutex::new(
        PairingWindow::new(Instant::now()).expect("window"),
    ));
    let code = window
        .lock()
        .expect("lock")
        .code(Instant::now())
        .expect("code")
        .to_string();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("first connection");
        let (payload, key) = transport::receive(&mut stream, Arc::clone(&window))
            .await
            .expect("authenticated transfer");
        assert_eq!(payload.as_slice(), b"private settings");
        transport::respond(&mut stream, key, true)
            .await
            .expect("acknowledgement");

        let (mut stream, _) = listener.accept().await.expect("second connection");
        assert!(transport::receive(&mut stream, window).await.is_err());
    });

    tokio::time::timeout(Duration::from_secs(30), async {
        let mut stream = TcpStream::connect(address).await.expect("connect");
        transport::send(&mut stream, &code, b"private settings")
            .await
            .expect("first transfer");
        let mut stream = TcpStream::connect(address).await.expect("reconnect");
        assert!(
            transport::send(&mut stream, &code, b"private settings")
                .await
                .is_err()
        );
        server.await.expect("server");
    })
    .await
    .expect("transfer timeout");
}
