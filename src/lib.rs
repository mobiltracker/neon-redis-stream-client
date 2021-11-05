use std::{
    error::Error,
    fmt::Formatter,
    sync::{Arc, Mutex},
};

use neon::prelude::*;
use redis::{Client, FromRedisValue};
use redis_stream_client::{sync::RedisStreamClient, sync::RedisStreamMessage};

pub struct RedisStreamWrapper {
    redis_stream_client: Arc<Mutex<RedisStreamClient>>,
    _redis_client: Mutex<Client>,
}

impl Finalize for RedisStreamWrapper {}

#[derive(Debug)]
// All errors will be converted to JavaScript exceptions with the `Display`
// implementation as the `Error` message.
struct StreamClientError(String);

impl From<redis::RedisError> for StreamClientError {
    fn from(err: redis::RedisError) -> Self {
        StreamClientError(format!("{:?}", err))
    }
}

impl std::fmt::Display for StreamClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&self.0)
    }
}

impl Error for StreamClientError {}

fn connect(mut cx: FunctionContext) -> JsResult<JsBox<RedisStreamWrapper>> {
    let connection_string = cx.argument::<JsString>(0)?.value(&mut cx);
    let stream_key = cx.argument::<JsString>(1)?.value(&mut cx);
    let consumer_group = cx.argument::<JsString>(2)?.value(&mut cx);
    let consumer_prefix = cx.argument::<JsString>(3)?.value(&mut cx);

    let redis_client = redis::Client::open(connection_string)
        .map_err(|err| StreamClientError::from(err))
        // Convert a Rust error to a JavaScript exception
        .or_else(|err| cx.throw_error(err.to_string()))?;

    let redis_connection = redis_client
        .get_connection()
        .map_err(|err| StreamClientError::from(err))
        .or_else(|err| cx.throw_error(err.to_string()))?;

    let stream_key: &'static str = Box::leak(Box::new(stream_key));
    let consumer_group: &'static str = Box::leak(Box::new(consumer_group));
    let consumer_prefix: &'static str = Box::leak(Box::new(consumer_prefix));

    let redis_stream_client = RedisStreamClient::new(
        redis_connection,
        stream_key,
        &consumer_group,
        &consumer_prefix,
    )
    .map_err(|err| {
        StreamClientError(format!(
            "Failed to connect to redis stream client: {}. Key:{} group:{}, prefix:{}",
            err, stream_key, consumer_group, consumer_prefix
        ))
    })
    .or_else(|err| cx.throw_error(err.to_string()))?;

    let wrapper = RedisStreamWrapper {
        _redis_client: redis_client.into(),
        redis_stream_client: Arc::new(Mutex::new(redis_stream_client)),
    };
    Ok(cx.boxed(wrapper))
}

fn read_next(mut cx: FunctionContext) -> JsResult<JsPromise> {
    // This is some funky syntax, but it's taking the first argument to the function,
    // attempting to downcast it as a `JsBox<StreamClient>`.
    let client_wrapper = &**cx.argument::<JsBox<RedisStreamWrapper>>(0)?;

    let stream_client = client_wrapper.redis_stream_client.clone();
    let promise = cx
        .task(
            move || -> Result<Option<RedisStreamMessage>, StreamClientError> {
                let mut client = stream_client
                    .lock()
                    .map_err(|_| StreamClientError("Poisoned Mutex".to_owned()))?;

                client
                    .read_next()
                    .map_err(|err| StreamClientError::from(err))
            },
        )
        .promise(resolve_get_next);

    Ok(promise)
}

fn ack_message(mut cx: FunctionContext) -> JsResult<JsPromise> {
    // This is some funky syntax, but it's taking the first argument to the function,
    // attempting to downcast it as a `JsBox<StreamClient>`.
    let client_wrapper = &**cx.argument::<JsBox<RedisStreamWrapper>>(0)?;
    let msg_id = cx.argument::<JsString>(1)?.value(&mut cx);

    let stream_client = client_wrapper.redis_stream_client.clone();
    let promise = cx
        .task(move || -> Result<(), StreamClientError> {
            let client = stream_client
                .lock()
                .map_err(|_| StreamClientError("Poisoned Mutex".to_owned()))?;

            Ok(client
                .ack_message_id(&msg_id)
                .map_err(|err| StreamClientError::from(err))?)
        })
        .promise(|cx, unit| {
            Ok(unit
                .and_then(|_| Ok(JsNull::new(cx)))
                .or_else(|err| cx.throw_error(err.to_string()))?)
        });

    Ok(promise)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("connect", connect)?;
    cx.export_function("readNext", read_next)?;
    cx.export_function("ackMessage", ack_message)?;
    Ok(())
}

fn resolve_get_next<'a>(
    cx: &mut TaskContext<'a>,
    // Return value from `cx.task(..)` closure
    result: Result<Option<RedisStreamMessage>, StreamClientError>,
) -> JsResult<'a, JsObject> {
    let output = result
        // An error may have occurred while compressing; conditionally grab the
        // written data
        .and_then(|msg| {
            if let Some(msg) = msg {
                let msg_object = JsObject::new(cx);
                let msg_key_value = JsString::new(cx, msg.key);
                let msg_key_id = JsString::new(cx, "key");

                msg_object
                    .set(cx, msg_key_id, msg_key_value)
                    .map_err(|_| StreamClientError("Error convering to obj".to_string()))?;

                for (key, val) in msg.inner_map {
                    let key = JsString::new(cx, key);
                    let val = JsString::new(cx, String::from_redis_value(&val).unwrap());
                    msg_object
                        .set(cx, key, val)
                        .map_err(|_| StreamClientError("Error convering to obj".to_string()))?;
                }

                Ok(msg_object)
            } else {
                Ok(JsObject::new(cx))
            }
        })
        // Convert a Rust error to a JavaScript exception
        .or_else(|err| cx.throw_error(err.to_string()))?;

    Ok(output)
}
