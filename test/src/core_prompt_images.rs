use ggok_core::parse::{
    image_caption_lines, prompt_body_with_files, prompt_file_is_image, with_image_captions,
};
use ggok_core::types::PromptFile;

fn img(path: &str) -> PromptFile {
    PromptFile {
        path: path.to_string(),
        mime: Some("image/png".into()),
    }
}

fn note(path: &str) -> PromptFile {
    PromptFile {
        path: path.to_string(),
        mime: Some("text/plain".into()),
    }
}

#[test]
fn prompt_file_is_image_by_mime_and_extension() {
    assert!(prompt_file_is_image(&img("/tmp/.ggok-uploads/a.png")));
    assert!(prompt_file_is_image(&PromptFile {
        path: "/tmp/.ggok-uploads/x.bin".into(),
        mime: Some("image/webp".into()),
    }));
    assert!(prompt_file_is_image(&PromptFile {
        path: "/tmp/.ggok-uploads/shot.JPG".into(),
        mime: None,
    }));
    assert!(!prompt_file_is_image(&note("/tmp/.ggok-uploads/a.txt")));
}

#[test]
fn captions_follow_attachment_order_not_filename() {
    let files = vec![
        img("/tmp/.ggok-uploads/image-1.png"),
        note("/tmp/.ggok-uploads/notes.txt"),
        img("/tmp/.ggok-uploads/image.png"),
    ];
    assert_eq!(
        image_caption_lines(&files),
        vec!["[Image #1] 图 1".to_string(), "[Image #2] 图 2".to_string()]
    );
}

#[test]
fn with_image_captions_prepends_and_is_idempotent() {
    let files = vec![
        img("/tmp/.ggok-uploads/paste-1.png"),
        img("/tmp/.ggok-uploads/paste-2.png"),
    ];
    let once = with_image_captions("看图 1 和图 2", &files);
    assert_eq!(once, "[Image #1] 图 1\n[Image #2] 图 2\n\n看图 1 和图 2");
    assert_eq!(with_image_captions(&once, &files), once);
    assert_eq!(
        with_image_captions("", &files),
        "[Image #1] 图 1\n[Image #2] 图 2"
    );
    assert_eq!(with_image_captions("hello", &[]), "hello");
}

#[test]
fn prompt_body_keeps_order_and_appends_tags() {
    let files = vec![
        img("/tmp/.ggok-uploads/image-1.png"),
        img("/tmp/.ggok-uploads/image.png"),
    ];
    let body = prompt_body_with_files("图 1 是草稿箱", &files, "/home/gfox/workspace");
    assert!(body.starts_with("[Image #1] 图 1\n[Image #2] 图 2\n\n图 1 是草稿箱"));
    let idx1 = body
        .find("@/tmp/.ggok-uploads/image-1.png")
        .expect("first path");
    let idx2 = body
        .find("@/tmp/.ggok-uploads/image.png")
        .expect("second path");
    assert!(idx1 < idx2, "tags must keep attachment order, got {body}");
}

#[test]
fn prompt_body_skips_duplicate_tags() {
    let files = vec![img("/tmp/.ggok-uploads/a.png")];
    let text = "see @/tmp/.ggok-uploads/a.png";
    let body = prompt_body_with_files(text, &files, "/tmp");
    assert_eq!(body.matches("@/tmp/.ggok-uploads/a.png").count(), 1);
}
