use std::io::Stdout;

use ratatui::Frame;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Rect, Layout, Constraint, Alignment};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;


static FIGURE: &str  = 
r"
     ;;;;;;;;;;;;;;;;;;;      
     ;;;;;;;;;;;;;;;;;;;      
     ;                 ;      
     ;                 ;      
     ;      (^o^)丿    ;      
     ;                 ;      
     ;                 ;      
     ;                 ;      
     ;                 ;      
,;;;;;            ,;;;;;      
;;;;;;            ;;;;;;      
`;;;;'            `;;;;'      ";

static SPINNER_FRAMES: [char; 8] = ['|', '/', '―', '\\', '|', '/', '―', '\\'];

pub struct LoadingWidget {
    label: String,
    frame: u16,
}

impl LoadingWidget {

    pub fn new(label: &str) -> Self {

        Self {
            label: format!("{} {}", label, SPINNER_FRAMES[0]),
            frame: 0
        }
    }

    pub fn draw(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        
        if area.height < 20 {
            frame.render_widget(Paragraph::new(self.label.as_str()).style(Style::default().fg(super::ACC_COLOR)).alignment(Alignment::Center), area);  
        }
        else {
            let h = if self.frame < 4 { self.frame } else { 8 - self.frame };
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Length(1 + h), Constraint::Length(area.height - 1 - h)])
                .margin(1)
                .split(area);
            
            self.label.pop();
            self.label.push(SPINNER_FRAMES[self.frame as usize]);
            self.frame = (self.frame + 1) % 8;
            frame.render_widget(Paragraph::new(self.label.as_str()).style(Style::default().fg(super::ACC_COLOR)).alignment(Alignment::Center), chunks[0]);
            frame.render_widget(Paragraph::new(FIGURE).style(Style::default().fg(super::ACC_COLOR)).alignment(Alignment::Center), chunks[1]); 
        }
    }
}


