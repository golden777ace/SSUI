use std::cell::RefCell;
use std::time::SystemTime;

use super::{apply_decl, apply_style_decl, kind_tag, strip_comments, Node, Props, Style, Tree};

#[derive(Clone, Copy, PartialEq)]
enum State {
    Base,
    Hover,
    Focus,
}

struct Step {
    tag: Option<String>,
    class: Option<String>,
    child: bool,
}

struct Rule {
    steps: Vec<Step>,
    state: State,
    spec: u32,
    order: usize,
    decls: Vec<(String, String)>,
}

thread_local! {
    static SEED: RefCell<Vec<(Style, Style, Style, Props)>> = RefCell::new(Vec::new());
    static STATIC_CSS: RefCell<String> = RefCell::new(String::new());
    static WATCH: RefCell<Option<(String, SystemTime)>> = RefCell::new(None);
    static WATCH_CSS: RefCell<String> = RefCell::new(String::new());
}

pub fn add_source(tree: &mut Tree, css: &str) {
    STATIC_CSS.with(|s| {
        let mut b = s.borrow_mut();
        b.push('\n');
        b.push_str(css);
    });
    recompute(tree);
}

pub fn set_source(tree: &mut Tree, css: &str) {
    STATIC_CSS.with(|s| *s.borrow_mut() = css.to_string());
    recompute(tree);
}

pub fn watch(tree: &mut Tree, path: &str) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let stamp = mtime(path);
    WATCH.with(|w| *w.borrow_mut() = Some((path.to_string(), stamp)));
    WATCH_CSS.with(|c| *c.borrow_mut() = text);
    recompute(tree);
}

pub fn poll(tree: &mut Tree) -> bool {
    let cur = WATCH.with(|w| w.borrow().as_ref().map(|(p, s)| (p.clone(), *s)));
    let (path, old) = match cur {
        Some(v) => v,
        None => return false,
    };
    let now = mtime(&path);
    if now == old {
        return false;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    WATCH.with(|w| *w.borrow_mut() = Some((path, now)));
    WATCH_CSS.with(|c| *c.borrow_mut() = text);
    recompute(tree);
    true
}

fn mtime(path: &str) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn recompute(tree: &mut Tree) {
    restore(tree);
    let mut source = STATIC_CSS.with(|s| s.borrow().clone());
    source.push('\n');
    source.push_str(&WATCH_CSS.with(|c| c.borrow().clone()));
    let rules = parse(&source);
    cascade(tree, &rules);
}

fn restore(tree: &mut Tree) {
    let count = tree.nodes.len();
    let stale = SEED.with(|s| s.borrow().len() != count);
    if stale {
        let snap: Vec<(Style, Style, Style, Props)> = tree
            .nodes
            .iter()
            .map(|n| (n.style, n.style_hover, n.style_focus, n.props))
            .collect();
        SEED.with(|s| *s.borrow_mut() = snap);
        return;
    }
    SEED.with(|s| {
        let seed = s.borrow();
        for (i, n) in tree.nodes.iter_mut().enumerate() {
            n.style = seed[i].0;
            n.style_hover = seed[i].1;
            n.style_focus = seed[i].2;
            n.props = seed[i].3;
        }
    });
}

fn cascade(tree: &mut Tree, rules: &[Rule]) {
    let count = tree.nodes.len();
    let mut expl_text: Vec<bool> = (0..count).map(|i| tree.nodes[i].style.text.is_some()).collect();
    let mut expl_wrap: Vec<bool> = (0..count).map(|i| tree.nodes[i].style.wrap.is_some()).collect();
    let mut order: Vec<usize> = (0..rules.len()).collect();
    order.sort_by_key(|&i| (rules[i].spec, rules[i].order));
    for &ri in &order {
        let rule = &rules[ri];
        for i in 0..count {
            if !matches_rule(tree, i, rule) {
                continue;
            }
            match rule.state {
                State::Base => {
                    for (k, v) in &rule.decls {
                        apply_decl(&mut tree.nodes[i], k, v);
                        if k == "color" {
                            expl_text[i] = true;
                        }
                        if k == "wrap" {
                            expl_wrap[i] = true;
                        }
                    }
                }
                State::Hover => {
                    for (k, v) in &rule.decls {
                        apply_style_decl(&mut tree.nodes[i].style_hover, k, v);
                    }
                }
                State::Focus => {
                    for (k, v) in &rule.decls {
                        apply_style_decl(&mut tree.nodes[i].style_focus, k, v);
                    }
                }
            }
        }
    }
    inherit(tree, &expl_text, &expl_wrap);
}

fn inherit(tree: &mut Tree, expl_text: &[bool], expl_wrap: &[bool]) {
    for i in 0..tree.nodes.len() {
        let parent = match tree.nodes[i].parent {
            Some(p) => p,
            None => continue,
        };
        let text = tree.nodes[parent.0].style.text;
        let wrap = tree.nodes[parent.0].style.wrap;
        if !expl_text[i] && tree.nodes[i].style.text.is_none() {
            tree.nodes[i].style.text = text;
        }
        if !expl_wrap[i] && tree.nodes[i].style.wrap.is_none() {
            tree.nodes[i].style.wrap = wrap;
        }
        let font = tree.nodes[parent.0].style.font;
        let size = tree.nodes[parent.0].style.size;
        if tree.nodes[i].style.font.is_none() {
            tree.nodes[i].style.font = font;
        }
        if tree.nodes[i].style.size.is_none() {
            tree.nodes[i].style.size = size;
        }
    }
}

fn matches_rule(tree: &Tree, index: usize, rule: &Rule) -> bool {
    let steps = &rule.steps;
    let mut k = match steps.len().checked_sub(1) {
        Some(v) => v,
        None => return false,
    };
    if !step_matches(&tree.nodes[index], &steps[k]) {
        return false;
    }
    let mut cur = tree.nodes[index].parent;
    while k > 0 {
        let direct = steps[k].child;
        k -= 1;
        if direct {
            let p = match cur {
                Some(p) => p,
                None => return false,
            };
            if !step_matches(&tree.nodes[p.0], &steps[k]) {
                return false;
            }
            cur = tree.nodes[p.0].parent;
        } else {
            loop {
                let p = match cur {
                    Some(p) => p,
                    None => return false,
                };
                let node = &tree.nodes[p.0];
                cur = node.parent;
                if step_matches(node, &steps[k]) {
                    break;
                }
            }
        }
    }
    true
}

fn step_matches(node: &Node, step: &Step) -> bool {
    if let Some(t) = &step.tag {
        if kind_tag(&node.kind) != t {
            return false;
        }
    }
    if let Some(c) = &step.class {
        if node.class_name.as_deref() != Some(c.as_str()) {
            return false;
        }
    }
    true
}

fn parse(css: &str) -> Vec<Rule> {
    let cleaned = strip_comments(css);
    let mut out = Vec::new();
    let mut rest = cleaned.as_str();
    let mut order = 0usize;
    while let Some(open) = rest.find('{') {
        let head = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        let close = match after.find('}') {
            Some(c) => c,
            None => break,
        };
        let body = &after[..close];
        rest = &after[close + 1..];
        let decls = parse_body(body);
        if head.is_empty() || decls.is_empty() {
            continue;
        }
        for one in head.split(',') {
            let one = one.trim();
            if one.is_empty() {
                continue;
            }
            if let Some(mut rule) = parse_selector(one) {
                rule.decls = decls.clone();
                rule.order = order;
                out.push(rule);
                order += 1;
            }
        }
    }
    out
}

fn parse_body(body: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(colon) = part.find(':') {
            let key = part[..colon].trim().to_lowercase();
            let value = part[colon + 1..].trim().to_string();
            if !key.is_empty() && !value.is_empty() {
                decls.push((key, value));
            }
        }
    }
    decls
}

fn parse_selector(text: &str) -> Option<Rule> {
    let spaced = text.replace('>', " > ");
    let mut steps: Vec<Step> = Vec::new();
    let mut child = false;
    let mut state = State::Base;
    let mut spec = 0u32;
    for tok in spaced.split_whitespace() {
        if tok == ">" {
            child = true;
            continue;
        }
        let (body, st) = split_state(tok);
        if st != State::Base {
            state = st;
            spec += 100;
        }
        let mut step = Step {
            tag: None,
            class: None,
            child,
        };
        child = false;
        let mut rest = body;
        if let Some(pos) = rest.find('.') {
            let (head, tail) = rest.split_at(pos);
            if !head.is_empty() && head != "*" {
                step.tag = Some(head.to_lowercase());
                spec += 10;
            }
            step.class = Some(tail[1..].to_string());
            spec += 100;
            rest = "";
        }
        if !rest.is_empty() && rest != "*" {
            step.tag = Some(rest.to_lowercase());
            spec += 10;
        }
        steps.push(step);
    }
    if steps.is_empty() {
        return None;
    }
    Some(Rule {
        steps,
        state,
        spec,
        order: 0,
        decls: Vec::new(),
    })
}

fn split_state(tok: &str) -> (&str, State) {
    match tok.split_once(':') {
        Some((name, st)) => {
            let state = match st.trim() {
                "hover" => State::Hover,
                "focus" => State::Focus,
                _ => State::Base,
            };
            (name.trim(), state)
        }
        None => (tok, State::Base),
    }
}