//! Ensure all uploader types are exported

use vkontakte::tools::uploaders::{
    AudioUploader, DocMessagesUploader, DocUploader, DocWallUploader, GraffitiUploader,
    PhotoChatFaviconUploader, PhotoFaviconUploader, PhotoMarketUploader, PhotoMessageUploader,
    PhotoToAlbumUploader, PhotoWallUploader, SpeechUploader, VideoUploader, VoiceMessageUploader,
};

#[test]
fn all_uploader_types_exist() {
    let _ = PhotoMessageUploader::new();
    let _ = PhotoWallUploader::new();
    let _ = PhotoFaviconUploader::new();
    let _ = PhotoMarketUploader::new();
    let _ = PhotoToAlbumUploader::new(1);
    let _ = PhotoChatFaviconUploader::new(1);
    let _ = DocUploader::new();
    let _ = DocMessagesUploader::new();
    let _ = DocWallUploader::new();
    let _ = VoiceMessageUploader::new();
    let _ = GraffitiUploader::new();
    let _ = AudioUploader::new();
    let _ = VideoUploader::new();
    let _ = SpeechUploader::new();
}
