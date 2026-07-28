use vkontakte::dispatch::dispenser::{FileStateDispenser, StateDispenser};
use vkontakte::tools::fsm::StatePeer;

#[tokio::test]
async fn file_dispenser_persists_across_instances() {
    let path = std::env::temp_dir().join(format!(
        "vkontakte_fsm_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);

    {
        let d = FileStateDispenser::open(&path).await.unwrap();
        d.set(StatePeer::new(42, "menu")).await.unwrap();
    }

    {
        let d = FileStateDispenser::open(&path).await.unwrap();
        let peer = d.get(42).await.unwrap().expect("state restored");
        assert_eq!(peer.state, "menu");
        let _ = d.delete(42).await;
    }

    let _ = std::fs::remove_file(&path);
}
