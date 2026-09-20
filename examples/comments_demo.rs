//! Example demonstrating review comments on slides
//!
//! Shows how to:
//! - Register comment authors on the presentation
//! - Attach comments to slides (author, text, anchor position)
//! - Save a deck that opens with visible review comments in PowerPoint
//!
//! Run with: cargo run --example comments_demo

use ppt_rs::{Presentation, SlideContent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pres = Presentation::new().title("Q3 Planning");

    // Authors are presentation-wide; the same name always maps to the same id.
    let alice = pres.add_comment_author("Alice Reviewer", "AR");
    let bob = pres.add_comment_author("Bob Author", "BA");

    let pres = pres
        .add_slide(
            SlideContent::new("Roadmap Overview")
                .add_bullet("Ship v1.0 in October")
                .add_bullet("Hire two engineers")
                // x/y is the comment anchor on the slide, in EMU
                .add_comment(alice, "Add a target date for the launch", 500_000, 300_000)
                .add_comment(bob, "Headcount ask already approved", 500_000, 1_200_000),
        )
        .add_slide(
            SlideContent::new("Budget")
                .add_bullet("Cloud spend flat year over year")
                .add_comment(alice, "Link the cost breakdown deck here", 800_000, 600_000),
        );

    pres.save("examples/output/comments_demo.pptx")?;
    println!(
        "Saved examples/output/comments_demo.pptx — open in PowerPoint to see the review comments."
    );
    Ok(())
}
