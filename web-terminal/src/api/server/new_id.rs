pub async fn new_id() -> String {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::SeqCst;
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    format!("Terminal {}", NEXT.fetch_add(1, SeqCst))
}
