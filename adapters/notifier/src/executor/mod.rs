use crate::repositories::{get_client, QueueMessage};

use myc_core::domain::entities::MessageSending;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use redis::{FromRedisValue, Value};
use tracing::error;

/// Consumes messages from the message queue
///
/// This function consumes messages from the message queue sending by smtp.
#[tracing::instrument(name = "consume_messages", skip(message_sending_repo))]
pub async fn consume_messages(
    queue_name: String,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<i32, MappedErrors> {
    let client = get_client().await;
    let mut connection = match client.get_connection() {
        Ok(conn) => conn,
        Err(err) => {
            return creation_err(format!(
                "Failed to connect to the message queue: {err}"
            ))
            .as_error()
        }
    };

    let processing_queue = format!("{}_processing_queue", queue_name);
    let error_queue = format!("{}_error_queue", queue_name);
    let max_retries = 3;
    let mut retries = 0;
    let mut processed_messages = 0;

    //
    // Consume the queue up to the end
    //
    loop {
        if retries >= max_retries {
            break;
        }

        //
        // Move the message from the main queue to the temporary queue to
        // guarantee that the message will be processed only once
        //
        let value: Value = match redis::cmd("RPOPLPUSH")
            .arg(queue_name.to_owned())
            .arg(processing_queue.to_owned())
            .query(&mut connection)
        {
            Ok(res) => res,
            Err(err) => {
                if err.is_cluster_error() {
                    return creation_err(format!(
                        "Failed to consume notification to the message queue: {err}"
                    ))
                    .as_error();
                }

                if err.is_io_error() {
                    return creation_err(format!(
                        "Failed to consume notification to the message queue: {err}"
                    ))
                    .as_error();
                }

                error!("Failed to consume notification to the message queue: {err}");

                break;
            }
        };

        //
        // Process the message
        //
        let message =
            match process_record(value, message_sending_repo.clone()).await {
                //
                // If the message processing failed, move the message to the
                // error queue and continue to the next message
                //
                Err(err) => {
                    error!("Failed to process message: {err}");

                    let _: Value = match redis::cmd("RPOPLPUSH")
                        .arg(processing_queue.to_owned())
                        .arg(error_queue.to_owned())
                        .query(&mut connection)
                    {
                        Ok(res) => res,
                        Err(err) => {
                            error!(
                            "Failed to move message to the error queue: {err}"
                        );

                            //
                            // Update retries counter and check if the maximum
                            // number of retries was reached
                            //
                            retries += 1;

                            //
                            // Continue to the next message
                            //
                            continue;
                        }
                    };

                    //
                    // Update retries counter and check if the maximum
                    // number of retries was reached
                    //
                    retries += 1;

                    continue;
                }
                Ok(res) => res,
            };

        //
        // If the message was processed successfully, remove it from the
        // temporary queue
        //
        if let Some(message) = message {
            processed_messages += 1;

            let _: Value = match redis::cmd("LREM")
                .arg(processing_queue.to_owned())
                .arg(message.to_owned())
                .query(&mut connection)
            {
                Ok(res) => res,
                Err(err) => {
                    error!("Failed to cleanup message: {message}");
                    error!("Failed to cleanup message: {err}");
                    continue;
                }
            };
        } else {
            break;
        }
    }

    Ok(processed_messages)
}

async fn process_record(
    record: Value,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<Option<String>, MappedErrors> {
    let message_str: String = if Value::Nil == record {
        return Ok(None);
    } else {
        let parsed_value = FromRedisValue::from_redis_value(&record);

        match parsed_value {
            Ok(val) => val,
            Err(err) => {
                return creation_err(format!(
                    "Failed to parse notification from the message queue: {err}"
                ))
                .as_error()
            }
        }
    };

    let message = match serde_json::from_str::<QueueMessage>(&message_str) {
        Ok(msg) => msg,
        Err(err) => {
            return creation_err(format!(
                "Failed to deserialize notification: {err}"
            ))
            .as_error()
        }
    };

    if let CreateResponseKind::NotCreated(_, _) =
        message_sending_repo.send(message.message).await?
    {
        return creation_err("Failed to send message")
            .with_exp_true()
            .as_error();
    }

    Ok(Some(message_str.to_string()))
}