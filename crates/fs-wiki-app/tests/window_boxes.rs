//! The pictures of the window show the state of every checkbox with the styles of the program (finding of 2026-11-14: the picture of
//! the window drew without Blinc's default look of its widgets, so an unticked box looked like a bare tick).
//!
//! The real program gets those styles from the runner of its window (`blinc_app` `windowed.rs`: `ua_styles::install()`, ahead of the
//! theme's own rules); the picture here puts the same `UA_STYLES` ahead of the program's style sheet (`Canvas::with_window_styles`).
//!
//! Pattern: pixel test on the picture of the real window over the real wiki; the rectangles of the boxes come from the layout of
//! the same element. Pictures are kept in the folder named by `FS_WIKI_SHOTS`.

#![cfg(target_os = "linux")]

mod page_rig;

use fs_components::blinc::layout::prelude::RenderTree;
use page_rig::rig;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // pixel positions of a window: small, not negative
fn to_px(v: f32) -> u32 {
    v.max(0.0) as u32
}

/// The user's cells and list, ticked and unticked, in the German window.
const BOXES: &str = "# Aufgaben\n\n| Name | Test |\n| --- | :---: |\n| Kal | - [ ] Test? |\n| Ada | [x] |\n\n- [ ] offen\n- [x] erledigt\n";

/// The window laid out as the picture lays it out: the same element, the style sheet of the program with Blinc's default look
/// ahead of it, the rules for layout applied.
fn window_layout(r: &page_rig::Rig, w: f32, h: f32) -> RenderTree {
    use fs_components::blinc::layout::css_parser::Stylesheet;
    let mut tree = RenderTree::from_element(&r.app.build(w, h));
    let bundle = fs_components_render::bundle(&r.app.styles());
    let vars = fs_components::blinc::theme::ThemeState::get().to_css_variable_map();
    let mut css = String::from(fs_components::blinc::layout::ua_styles::UA_STYLES);
    for sheet in &bundle.css_sources {
        css.push('\n');
        css.push_str(sheet);
    }
    tree.set_stylesheet(Stylesheet::parse_with_variables(&css, &vars).expect("style sheet"));
    tree.apply_stylesheet_layout_overrides();
    tree.apply_stylesheet_base_styles();
    tree.compute_layout(w, h);
    tree
}

/// (x, y, width, height) of the element with this id in the window of `width` x `height`.
fn bounds_of(tree: &RenderTree, id: &str) -> (u32, u32, u32, u32) {
    let node = tree
        .query_by_id(id)
        .unwrap_or_else(|| panic!("no element {id}"));
    let b = tree.get_absolute_bounds(node).expect("bounds");
    (to_px(b.x), to_px(b.y), to_px(b.width), to_px(b.height))
}

#[test]
fn the_picture_shows_the_state_of_every_box_with_the_styles_of_the_program() {
    let _g = page_rig::serial();
    let r = rig(true);
    r.make("aufgaben.md", BOXES);
    r.open("aufgaben.md");
    r.warm_up();
    let tree = window_layout(&r, 1000.0, 700.0);
    let dir =
        std::env::var_os("FS_WIKI_SHOTS").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("wiki-0012-kaestchen.png");
    if r.window_picture(1000, 700, &file).is_err() {
        eprintln!("skipped: no graphics");
        return;
    }
    let img = Img::load(&file);
    // Boxes in document order: 0 and 1 are the cells (open, done), 2 and 3 the list (open, done).
    let blue = |p: &[u8; 3]| p[2] > 170 && p[0] < 120;
    let dark = |p: &[u8; 3]| p[0] < 100 && p[1] < 100 && p[2] < 100;
    for (index, done) in [(0, false), (1, true), (2, false), (3, true)] {
        let (left, top, wide, high) = bounds_of(&tree, &format!("fs-tv-task-{index}"));
        let region =
            || (left..left + wide).flat_map(move |i| (top..top + high).map(move |j| (i, j)));
        let blues = region()
            .filter(|&(i, j)| blue(&img.get_pixel(i, j)))
            .count();
        let darks = region()
            .filter(|&(i, j)| dark(&img.get_pixel(i, j)))
            .count();
        // The frame: on each of the four sides, within a pixel of the edge, a pixel differs from the ground beside the box
        // (a bare tick has no frame), in every state.
        let ground = img.get_pixel(left.saturating_sub(6), top + high / 2);
        let differs = |i: u32, j: u32| {
            let p = img.get_pixel(i, j);
            (0..3).any(|c| p[c].abs_diff(ground[c]) >= 25)
        };
        let (mx, my) = (left + wide / 2, top + high / 2);
        let sides = [
            (left.saturating_sub(1)..=left + 1).any(|i| differs(i, my)),
            (left + wide - 2..=left + wide).any(|i| differs(i, my)),
            (top.saturating_sub(1)..=top + 1).any(|j| differs(mx, j)),
            (top + high - 2..=top + high).any(|j| differs(mx, j)),
        ];
        let framed = sides.iter().all(|s| *s);
        assert!(
            framed,
            "box {index}: no frame (the picture {})",
            file.display()
        );
        if done {
            assert!(
                blues * 2 >= (wide * high) as usize,
                "box {index} is done: filled ({blues} of {} px)",
                wide * high
            );
        } else {
            assert_eq!(blues, 0, "box {index} is open: not filled");
            assert_eq!(darks, 0, "box {index} is open: no tick inside");
            let ink = (left + 3..left + wide - 3)
                .flat_map(|i| (top + 3..top + high - 3).map(move |j| (i, j)))
                .filter(|&(i, j)| img.get_pixel(i, j).iter().any(|c| *c < 235))
                .count();
            assert_eq!(ink, 0, "box {index} is open: the inside is empty");
        }
    }
}

/// A PNG read into memory.
struct Img {
    width: u32,
    step: usize,
    data: Vec<u8>,
}

impl Img {
    fn load(file: &std::path::Path) -> Self {
        let decoder =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(file).unwrap()));
        let mut reading = decoder.read_info().unwrap();
        let mut data = vec![0; reading.output_buffer_size().unwrap()];
        let info = reading.next_frame(&mut data).unwrap();
        Self {
            width: info.width,
            step: info.color_type.samples(),
            data,
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let at = (y * self.width + x) as usize * self.step;
        [self.data[at], self.data[at + 1], self.data[at + 2]]
    }
}
