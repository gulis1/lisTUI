pub mod list;
pub mod player;
pub mod loading;

use std::io::Stdout;

use ratatui::style::{Style, Color,};
use ratatui::widgets::{Paragraph, Block, Borders,BorderType};
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use ratatui::layout::{Rect, Alignment, Layout, Constraint};
use lazy_static::lazy_static;


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


static FIGURE: &str  = 
r"
     ;;;;;;;;;;;;;;;;;;;      
     ;;;;;;;;;;;;;;;;;;;      
     ;                 ;      
     ;                 ;      
     ;     (⋟ ﹏ ⋞)    ;      
     ;                 ;      
     ;                 ;      
     ;                 ;      
     ;                 ;      
,;;;;;            ,;;;;;      
;;;;;;            ;;;;;;      
`;;;;'            `;;;;'      ";

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


pub fn draw_controls_screen(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(CONTROLS)
        .block(BLOCK.clone().title("Controls"))
        .alignment(Alignment::Left);

    frame.render_widget(p, area);
}

pub fn draw_error_msg(frame: &mut Frame<CrosstermBackend<Stdout>>, msg: &str) {

    if frame.size().height < 20 {
        frame.render_widget(Paragraph::new(msg).style(Style::default().fg(Color::Red)).alignment(Alignment::Center), frame.size());  
    }
    else {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Length(frame.size().height - 4)])
            .margin(1)
            .split(frame.size());
        
        frame.render_widget(Paragraph::new(msg).style(Style::default().fg(Color::Red)).alignment(Alignment::Center), chunks[0]);
        frame.render_widget(Paragraph::new(FIGURE).style(Style::default().fg(Color::Red)).alignment(Alignment::Center), chunks[1]); 
    }
}

pub fn draw_logo(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(LOGO)
        .alignment(Alignment::Center)
        .style(Style::default().fg(ACC_COLOR));

    frame.render_widget(p, area);
}
