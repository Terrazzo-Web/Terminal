use futures::Sink;
use web_sys::js_sys;

pub fn to_json_sink<I>() -> (String, impl Sink<I, Error = std::io::Error>) {
    let correlation_id = format!("X{}", js_sys::Math::random());
    (
        correlation_id,
        futures::sink::unfold(0, async |s, i| {
            let _ = i;
            Ok(s + 1)
        }),
    )
}
