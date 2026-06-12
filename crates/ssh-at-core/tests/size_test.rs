// Test HostEntry size
use ssh_at_core::config::HostEntry;
use std::mem;

#[test]
fn test_hostentry_size() {
    eprintln!("[SIZE_TEST] HostEntry size: {} bytes", mem::size_of::<HostEntry>());
    eprintln!("[SIZE_TEST] HostEntry align: {} bytes", mem::align_of::<HostEntry>());

    // Create one on stack
    let entry = HostEntry::new("test".to_string());
    eprintln!("[SIZE_TEST] ✅ Stack allocation succeeded");
    assert_eq!(entry.host, "test");

    // Create Vec with many entries
    let mut entries = Vec::new();
    for i in 0..100 {
        entries.push(HostEntry::new(format!("host{}", i)));
    }
    eprintln!("[SIZE_TEST] ✅ Vec with {} entries", entries.len());
    assert_eq!(entries.len(), 100);
}
