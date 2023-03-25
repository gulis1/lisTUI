use std::io::Stdout;
use listui_lib::models::Drawable;
use tui::style::{Style, Color, Modifier};
use tui::text::Span;
use tui::widgets::{ListState, Paragraph, Block, Borders, ListItem, List, BorderType};
use tui::backend::CrosstermBackend;
use tui::Frame;
use tui::layout::{Rect, Alignment};
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

static CONTROLS: &str =
"\
↵    play.
↑/↓  select.  
←/→  jump 5s.  
F    follow mode.
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

// Generic list widget, that support drawing a filtered view of itself.
// The filtering is only computed when the search query changes.
pub(super) struct ListWidget<T: Drawable> {

    state: ListState,
    items: Vec<T>,
    
    filtered: bool,
    last_query: Option<String>,
    filtered_indexes: Vec<usize>,
    filter_state: ListState
}

impl<T: Drawable> ListWidget<T> {

    pub fn empty() -> Self {

        Self {
            state: ListState::default(),
            items: Vec::new(),

            last_query: None,
            filtered_indexes: Vec::new(),
            filter_state: ListState::default(),
            filtered: false
        }
    }
    
    pub fn with_items(items: Vec<T>) -> Self {

        Self {
            state: ListState::default(),
            items,

            last_query: None,
            filtered_indexes: Vec::new(),
            filter_state: ListState::default(),
            filtered: false
        }
    }

    pub fn get_selected(&self) -> Option<usize> {

        if self.filtered {
            self.filter_state.selected().map(|ind| self.filtered_indexes[ind])
        }
        else { self.state.selected() }
    }

    pub fn next(&mut self) {
        
        let st = if self.filtered { &mut self.filter_state } else { &mut self.state };
        let len = if self.filtered { self.filtered_indexes.len() } else { self.items.len() };
        
        if len > 0 {

            let next = match st.selected() {
                Some(i) => { (i + 1) % len },
                None => 0,
            };
            st.select(Some(next));
        }   
    }

    pub fn previous(&mut self) {
        
        let st = if self.filtered { &mut self.filter_state } else { &mut self.state };
        let len = if self.filtered { self.filtered_indexes.len() } else { self.items.len()} ;

        if len > 0 {

            let i = match st.selected() {
                Some(i) => {
                    if i == 0 { len - 1 } 
                    else { i - 1 }
                }
                None => 0,
            };
            st.select(Some(i));
        }
    }

    pub fn select_ind(&mut self, ind: usize) {

        let st = if self.filtered { &mut self.filter_state } else { &mut self.state };
        let len = if self.filtered { self.filtered_indexes.len() } else { self.items.len() };

        if ind < len {
            st.select(Some(ind));
        }
    }

    pub fn filter(&mut self, query: &str) {

        // self.last_query cannot be none is self.filtered is true
        // so using unwrap shuold be safe here.
        if !self.filtered || self.last_query.as_ref().unwrap() != query {

            self.filtered_indexes = self.items.iter()
                .enumerate()
                .filter(|(_, t)| t.get_text().to_ascii_lowercase().contains(query))
                .map(|(ind, _)| ind)
                .collect();

            self.filter_state = ListState::default();
            self.last_query = Some(String::from(query));
            self.filtered = true;
        }    
    }

    pub fn clear_filter(&mut self) {
        self.filtered = false;
    }

    pub fn is_filtered(&self) -> bool {
        self.filtered
    }

    pub fn draw(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, title: &str) {
        
        if self.filtered { self.draw_filtered(frame, area, title); }
        else { self.draw_all(frame, area, title); }
    }

    fn draw_all(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, title: &str) {

        let items: Vec<ListItem> = self.items
            .iter()
            .map(|p| {
                let lines = Span::from(p.get_text());
                ListItem::new(lines).style(Style::default())
            })
            .collect();
        
        let list = List::new(items)
            .block(BLOCK.clone().title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(ACC_COLOR)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }

    fn draw_filtered(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, title: &str)  {

        let filtered: Vec<ListItem> = self.filtered_indexes
            .iter()
            .map(|ind| {

                let lines = Span::from(self.items[*ind].get_text());
                ListItem::new(lines).style(Style::default())
            })
            .collect();
 
        let list = List::new(filtered)
            .block(BLOCK.clone().title(title))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(ACC_COLOR)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.filter_state);
    }

    pub fn get_ind(&self, ind: usize) -> &T {
        &self.items[ind]
    }

    pub fn total_len(&self) -> usize {
        self.items.len()
    }
}

pub fn draw_controls_screen(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(CONTROLS)
        .block(BLOCK.clone().title("Controls"))
        .alignment(Alignment::Left);

    frame.render_widget(p, area);
}

pub fn draw_not_enough_height(frame: &mut Frame<CrosstermBackend<Stdout>>) {

    let p = Paragraph::new("Please make the terminal a bit taller :(")
        .alignment(Alignment::Center);

    frame.render_widget(p, frame.size());
}

pub fn draw_not_enough_width(frame: &mut Frame<CrosstermBackend<Stdout>>) {

    let p = Paragraph::new("-->(x_x)<--")
        .alignment(Alignment::Center);

    frame.render_widget(p, frame.size());
}

pub fn draw_logo(frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

    let p = Paragraph::new(LOGO)
        .alignment(Alignment::Center)
        .style(Style::default().fg(ACC_COLOR));

    frame.render_widget(p, area);
}