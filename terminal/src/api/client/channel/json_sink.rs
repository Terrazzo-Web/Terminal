use futures::Sink;

pub fn to_json_sink<I>() -> impl Sink<I, Error = std::io::Error> {
    futures::sink::unfold(0, async |s, i| {
        let _ = i;
        Ok(s + 1)
    })
}
