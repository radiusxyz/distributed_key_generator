pub async fn wait_for_decryption_key<C: Config>(
    ctx: &C,
    session_id: SessionId,
    timeout_secs: u64,
) -> Result<DecryptionKey, C::Error> {
    let poll_interval = Duration::from_secs(1);
    let mut waited = 0;
    loop {
        match DecryptionKey::get(session_id) {
            Ok(key) => {
                info!("{} Received decryption key on session {:?}", ctx.log_prefix(), session_id);
                return Ok(key);
            }
            Err(_) => {
                if waited >= timeout_secs {
                    error!("{} Timeout waiting for decryption key on session {:?}", ctx.log_prefix(), session_id);
                    return Err(C::Error::from(RpcClientError::Response(format!(
                        "Solver did not submit decryption key for session {:?} in time",
                        session_id
                    ))));
                }

                debug!(
                    "{} Still waiting for decryption key on session {:?} (waited: {}s)",
                    ctx.log_prefix(), session_id, waited
                );

                sleep(poll_interval).await;
                waited += 1;
            }
        }
    }
}