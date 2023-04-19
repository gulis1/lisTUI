pub mod list;
pub mod player;

use std::io::Stdout;

use tui::style::{Style, Color,};
use tui::widgets::{Paragraph, Block, Borders,BorderType};
use tui::backend::CrosstermBackend;
use tui::Frame;
use tui::layout::{Rect, Alignment};
use lazy_static::lazy_static;


pub fn draw_controls_screen(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(CONTROLS)
        .block(BLOCK.clone().title("Controls"))
        .alignment(Alignment::Left);

    frame.render_widget(p, area);
}

pub fn draw_error_msg(frame: &mut Frame<CrosstermBackend<Stdout>>, msg: &str) {

    let p = Paragraph::new(msg)
        .alignment(Alignment::Center);

    frame.render_widget(p, frame.size());
}

pub fn draw_logo(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(LOGO)
        .alignment(Alignment::Center)
        .style(Style::default().fg(ACC_COLOR));

    frame.render_widget(p, area);
}


static LOGO: &str =
r"
$$\ $$\        $$$$$$$$\ $$\   $$\ $$$$$$\ 
$$ |\__|       \__$$  __|$$ |  $$ |\_$$  _|
$$ |$$\  $$$$$$$\ $$ |   $$ |  $$ |  $$ |  
$$ |$$ |$$  _____|$$ |   $$ |  $$ |  $$ |  
$$ |$$ |\$$$$$$\  $$ |   $$ |  $$ |  $$ |  
$$ |$$ | \____$$\ $$ |   $$ |  $$ |  $$ |  
$$ |$$ |$$$$$$$  |$$ |   \$$$$$$  |$$$$$$\ 
\__|\__|\_______/ \__|    \______/ \______|";

static CONTROLS: &str =
"\
↵    play.
↑/↓  select.  
←/→  jump 5s.  
F    follow mode.
N    play next.
B    play previous.
P    pause (ESC to cancel).
S    search.
R    toffle shuffle.
Q    go back to last screen.

Press any key to close this screen.";

// Accent color.
pub const ACC_COLOR: Color = Color::LightBlue;
lazy_static! {
    
    // Default block.
    pub static ref BLOCK: Block<'static> = {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACC_COLOR))
    };
}
