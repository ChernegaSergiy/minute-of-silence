//! NTP query tests.
//!
//! Legacy functions moved to ntp_service.rs.

#[cfg(test)]
mod tests {
    /// Requires outbound UDP to port 123. Run explicitly with:
    ///   cargo test -- --ignored rsntp_library_works
    #[test]
    #[ignore]
    fn rsntp_library_works() {
        // Use a more stable NTP server for CI tests
        let result = rsntp::SntpClient::new().synchronize("time.google.com");

        if let Err(e) = result {
            println!("NTP query failed, trying alternative: {:?}", e);
            let alt_result = rsntp::SntpClient::new().synchronize("time.cloudflare.com");
            assert!(
                alt_result.is_ok(),
                "Both NTP servers (Google and Cloudflare) failed: {:?}",
                alt_result.err()
            );
        } else {
            assert!(result.is_ok());
        }
    }
}
