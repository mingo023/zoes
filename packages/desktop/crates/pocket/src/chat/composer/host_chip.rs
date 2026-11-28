use super::{ChatBoxEvent, ComposerState};
use crate::composer::Menu;
use crate::desktop::Desktop;
use crate::spaces::Space;
use crate::spaces::header::tile;
use agents::Summary;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use theme::*;

const WIDTH: f32 = 288.;

pub(crate) struct HostChip {
    agent: String,
    here: String,
    mode: &'static str,
    status: Option<ui::Notice>,
    hosts: Vec<Dest>,
}

struct Dest {
    id: String,
    name: String,
    address: String,
    glyph: &'static str,
    colour: Option<u32>,
}

#[derive(Clone)]
pub(crate) enum Move {
    To(String),
    Show,
    AddHost,
}

fn mode(tree: Option<&git::Worktree>) -> &'static str {
    if tree.is_some_and(|t| !t.main) { "Worktree" } else { "Local" }
}

impl Desktop {
    pub(super) fn chat_host_chip(&self, agent: &Summary, cx: &App) -> Option<HostChip> {
        if !self.can_hand_off(agent) {
            return None;
        }
        let order = self.space_order();
        let hosts = self
            .fleet
            .hosts
            .list
            .iter()
            .filter(|h| self.fleet.hosts.links.online(&h.id))
            .map(|h| {
                let space = Space::on(Some(&h.id));
                Dest {
                    id: h.id.clone(),
                    name: self.host_name(&h.id),
                    address: h.address(),
                    glyph: self.space_glyph(&space),
                    colour: self.space_colour(&space, &order),
                }
            })
            .collect();
        Some(HostChip {
            agent: agent.id.clone(),
            here: self.space_name(&Space::Local),
            mode: mode(self.worktree_of(&agent.cwd)),
            status: self.handoff_notice(&agent.id, cx),
            hosts,
        })
    }

    pub(super) fn on_move(&mut self, agent: &str, m: &Move, window: &mut Window, cx: &mut Context<Self>) {
        match m {
            Move::To(host) => self.open_handoff(agent, Some(host.clone()), window, cx),
            Move::Show => self.open_handoff(agent, None, window, cx),
            Move::AddHost => self.show_hosts(window, cx),
        }
    }
}

impl ComposerState {
    pub(super) fn host_chip(&self, chip: &HostChip, cx: &mut Context<Self>) -> Div {
        let open = self.draft.menu == Some(Menu::Host);
        let (lead, label, ink) = match &chip.status {
            Some(n) => {
                (icon(n.icon.unwrap_or("server"), 13., if n.warn { WAITING } else { TEXT_3 }), n.text.clone(), if n.warn { WAITING_TEXT } else { TEXT_2 })
            }
            None => (icon("laptop", 13., TEXT_3), chip.here.clone().into(), TEXT_2),
        };
        let trigger = ui::popover_trigger(
            div()
                .id("chat-host")
                .h(px(22.))
                .px(px(6.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|d| d.bg(FILL_2))
                .when(open, |d| d.bg(FILL_2))
                .child(lead)
                .child(div().max_w(px(220.)).truncate().text_color(ink).child(label))
                .child(icon(if open { "chevron-down" } else { "chevron-up" }, 11., TEXT_4)),
            open,
            cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.draft.toggle(Menu::Host);
                cx.notify();
            }),
        );
        let menu = open.then(|| self.host_menu(chip, cx));
        div().relative().flex_none().ml(px(-6.)).child(trigger).children(menu)
    }

    fn host_menu(&self, chip: &HostChip, cx: &mut Context<Self>) -> Div {
        let act = |row: Stateful<Div>, to: Move, cx: &mut Context<Self>| {
            let agent = chip.agent.clone();
            row.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.close_menu(cx);
                cx.emit(ChatBoxEvent::Move { agent: agent.clone(), to: to.clone() });
            }))
        };
        let head = div()
            .px(px(10.))
            .pt(px(6.))
            .pb(px(7.))
            .flex()
            .flex_col()
            .child(div().text_size(px(13.5)).text_color(TEXT).child(chip.here.clone()))
            .child(div().text_size(px(12.)).text_color(TEXT_3).child(chip.mode));
        let mut rows: Vec<AnyElement> = vec![head.into_any_element(), ui::menu_divider().into_any_element()];
        if chip.status.is_some() {
            rows.push(act(ui::menu_row("chat-host-show", "server", crate::handoff::menu_label(true), None), Move::Show, cx).into_any_element());
        } else {
            if !chip.hosts.is_empty() {
                rows.push(ui::menu_label("Move to").into_any_element());
            }
            for h in &chip.hosts {
                let row = div()
                    .id(SharedString::from(format!("chat-host-to-{}", h.id)))
                    .px(px(8.))
                    .py(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(|d| d.bg(FILL_2))
                    .child(tile(h.glyph, h.colour, 26.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().text_size(px(13.5)).text_color(TEXT).child(h.name.clone()))
                            .child(div().truncate().text_size(px(12.)).text_color(TEXT_3).child(h.address.clone())),
                    );
                rows.push(act(row, Move::To(h.id.clone()), cx).into_any_element());
            }
            if !chip.hosts.is_empty() {
                rows.push(ui::menu_divider().into_any_element());
            }
            rows.push(act(ui::menu_row("chat-host-add", "plus", "Add a host…", None), Move::AddHost, cx).into_any_element());
        }
        let pop = ui::pop(div().id("chat-host-menu")).w(px(WIDTH)).p(px(6.)).flex().flex_col().occlude().children(rows).on_mouse_down_out(cx.listener(
            |this, _: &MouseDownEvent, _, cx| {
                this.close_menu(cx);
            },
        ));
        div().absolute().top(px(-6.)).left_0().child(
            deferred(anchored().anchor(Anchor::BottomLeft).snap_to_window_with_margin(px(8.)).child(ui::menu_in("chat-host-menu-in", pop))).with_priority(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::mode;

    fn tree(main: bool) -> git::Worktree {
        git::Worktree { path: "/code/app".into(), branch: "main".into(), main, made: false, created: None }
    }

    #[test]
    fn a_session_in_a_worktree_of_its_own_reads_worktree_and_one_in_the_projects_checkout_reads_local() {
        assert_eq!(mode(Some(&tree(false))), "Worktree");
        assert_eq!(mode(Some(&tree(true))), "Local");
        assert_eq!(mode(None), "Local", "a folder no tracked tree holds");
    }
}
