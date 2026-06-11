use super::*;

#[test]
fn waveform_loading_visual_paints_full_height_gray_fill_without_chrome() {
    let frame = waveform_loading_visual("kick.wav", 0.25)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 172.0));

    let fill_rects = frame.paint_plan.fill_rects().collect::<Vec<_>>();

    assert!(fill_rects.iter().any(|fill| {
        (fill.rect.width() - 180.0).abs() < 0.01
            && (fill.rect.height() - 172.0).abs() < 0.01
            && fill.rect.min.x == 0.0
            && fill.rect.min.y == 0.0
            && fill.color.r == 174
            && fill.color.g == 178
            && fill.color.b == 181
    }));
    assert!(
        frame.paint_plan.stroke_rects().next().is_none()
            && frame.paint_plan.text_runs().next().is_none()
    );
}
