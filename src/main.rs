#![feature(option_result_contains)]

use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};

#[derive(Serialize, Deserialize, Debug)]
struct I3Tree {
    window: Option<u64>,
    r#type: String,
    name: Option<String>,
    nodes: Vec<I3Tree>,
    floating_nodes: Vec<I3Tree>,
}

#[derive(Debug)]
struct Node {
    id: Option<u64>,
    kind: String,
    name: Option<String>,
    children: Vec<Node>,
}

impl From<I3Tree> for Node {
    fn from(mut tree: I3Tree) -> Self {
        let mut children = tree.nodes;
        children.append(&mut tree.floating_nodes);
        Node {
            id: tree.window,
            kind: tree.r#type,
            name: tree.name,
            children: children.into_iter().map(Node::from).collect(),
        }
    }
}

impl Node {
    fn into_leaves(self) -> Vec<Leaf> {
        if self.children.len() == 0 {
            vec![Leaf {
                id:   self.id,
                kind: self.kind,
                name: self.name,
            }]
        } else {
            let mut leaves = vec![];
            for child in self.children {
                leaves.append(&mut child.into_leaves());
            }

            leaves
        }
    }

    fn find_name(self, name: &str) -> Option<Node> {
        if self.name.contains(&name) {
            Some(self)
        } else {
            self.children.into_iter().find_map(|n| n.find_name(name))
        }
    }
}

#[derive(Debug)]
struct Leaf {
    id:   Option<u64>,
    kind: String,
    name: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let dmenu = &std::env::args().nth(1).unwrap_or("dmenu".to_owned());
    let out = Command::new("i3-msg").args(&["-t", "get_tree"]).output()?;
    let tree: I3Tree = serde_json::from_reader(out.stdout.as_slice())?;
    let tree: Node = tree.into();
    let mut scratch_windows = tree.find_name("__i3_scratch").unwrap().into_leaves();
    scratch_windows.sort_unstable_by(|a, b| Option::cmp(&a.name,&b.name));
    let scratch_windows: Vec<_> = scratch_windows
        .into_iter()
        .filter(|l| l.kind == "con")
        .enumerate()
        .map(|(i, leaf)| {
            (
                leaf.id.unwrap(),
                format!(
                    "{}:{}",
                    i + 1,
                    leaf.name
                        .as_ref()
                        .map(String::as_ref)
                        .unwrap_or("<No title>")
                ),
            )
        })
        .collect();

    let mut dmenu = Command::new("sh")
        .args(&["-c", dmenu])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdin = dmenu.stdin.as_mut().unwrap();
    stdin.write_all(&scratch_windows.iter().map(|(_, name)| name).fold(
        vec![],
        |mut vec, name| {
            vec.append(&mut format!("{}\n", name).into_bytes());
            vec
        },
    ))?;

    let choice = String::from_utf8(dmenu.wait_with_output()?.stdout)?;
    let choice = choice.trim();
    if choice.is_empty() {
        // Selection aborted by user
        return Ok(());
    }

    dbg!(&choice);
    let id = dbg!(&scratch_windows)
        .iter()
        .find_map(|(id, name)| if *name == choice { Some(id) } else { None })
        .unwrap();
    Command::new("i3-msg")
        .args(&[
            "-t",
            "command",
            &format!("[id={0}] scratchpad show; [id={0}] floating disable", id),
        ])
        .spawn()
        .unwrap()
        .wait()?;

    Ok(())
}
