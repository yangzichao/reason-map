//! Prompt construction. The graph is serialized into a compact, token-frugal outline so
//! Claude has the full argument as context for every operation (SPEC §7.9).

use crate::domain::{Edge, MapGraph, Node};

/// Compact textual rendering of a node: `[id | status] text`.
fn node_line(n: &Node) -> String {
    format!("[{} | {}] {}", n.id, n.status.as_str(), n.text)
}

fn edge_line(e: &Edge) -> String {
    let strength = e
        .strength
        .map(|s| format!(" ({})", s.as_str()))
        .unwrap_or_default();
    format!("{} --{}{}-> {}", e.from_node, e.edge_type.as_str(), strength, e.to_node)
}

/// Render the whole map (or a focused subset) as context.
pub fn serialize_context(graph: &MapGraph, focus: &[String]) -> String {
    let mut out = String::new();
    out.push_str("ARGUMENT MAP\n");
    out.push_str("Nodes (id | status):\n");
    for n in &graph.nodes {
        out.push_str("  ");
        out.push_str(&node_line(n));
        out.push('\n');
    }
    out.push_str("Reasoning edges (support/rebut/premise_of/depends_on):\n");
    if graph.edges.is_empty() {
        out.push_str("  (none yet)\n");
    }
    for e in &graph.edges {
        out.push_str("  ");
        out.push_str(&edge_line(e));
        out.push('\n');
    }
    if !focus.is_empty() {
        out.push_str("Currently selected node ids: ");
        out.push_str(&focus.join(", "));
        out.push('\n');
    }
    out
}

pub const SHARED_STANCE: &str = "\
You are a reasoning partner inside an argument-mapping tool. The user maps claims that are \
NOT ironclad — they contain assumptions, data, and bets. Your job is to help advance and \
stress-test the argument. Be concrete and specific to THIS map; never generic. The human \
is always the judge — you propose, you do not decide.";

pub fn forward_inference_prompt(graph: &MapGraph, focus: &[String]) -> (String, String) {
    let system = format!(
        "{SHARED_STANCE}\n\nTask: forward inference. Given the selected node(s), propose 2-3 \
         distinct claims that could be derived NEXT (downstream). Each must follow plausibly \
         from the selection plus the map. Mark each suggested_status as one of \
         fact|assumption|bet|evidenced|open.\n\n\
         Respond with ONLY a JSON object: \
         {{\"suggestions\":[{{\"text\":\"...\",\"rationale\":\"...\",\"suggestedStatus\":\"bet\"}}]}}"
    );
    let user = serialize_context(graph, focus);
    (system, user)
}

pub fn gap_detection_prompt(graph: &MapGraph, from_id: &str, to_id: &str) -> (String, String) {
    let system = format!(
        "{SHARED_STANCE}\n\nTask: gap detection. The user claims node {from_id} leads to node \
         {to_id}, but the step may be too big. Identify the missing intermediate claim(s) / \
         hidden premise(s) needed to make the inference hold. Return 1-3, ordered.\n\n\
         Respond with ONLY a JSON object: \
         {{\"gaps\":[{{\"text\":\"...\",\"rationale\":\"...\"}}]}}"
    );
    let user = serialize_context(graph, &[from_id.to_string(), to_id.to_string()]);
    (system, user)
}

pub fn challenge_prompt(
    graph: &MapGraph,
    target_kind: &str,
    target_id: &str,
    diverse: bool,
) -> (String, String) {
    let lens = if diverse {
        "Produce 3-4 attacks, each from a DIFFERENT angle (use distinct kinds: rebuttal, \
         counterexample, hidden_assumption, alternative, non_sequitur). Do not pile on one angle."
    } else {
        "Produce 1-2 of the strongest attacks."
    };
    let system = format!(
        "{SHARED_STANCE}\n\nTask: red-team / adversarial attack. Attack the {target_kind} with id \
         {target_id}. Attacking a node = argue the claim is false/unfounded; attacking an edge = \
         argue the inference step does not hold. {lens} Each attack must be specific and \
         defeasible — something the user can actually weigh and answer.\n\n\
         Respond with ONLY a JSON object: \
         {{\"challenges\":[{{\"kind\":\"hidden_assumption\",\"content\":\"...\"}}]}}"
    );
    let user = serialize_context(graph, &[target_id.to_string()]);
    (system, user)
}

pub fn weak_points_prompt(graph: &MapGraph) -> (String, String) {
    let system = format!(
        "{SHARED_STANCE}\n\nTask: scan the whole map and identify the nodes most worth attacking \
         — the load-bearing bets and assumptions whose failure would collapse the most. Return \
         up to 5, most fragile first, each referencing an existing node id.\n\n\
         Respond with ONLY a JSON object: \
         {{\"weakPoints\":[{{\"nodeId\":\"...\",\"reason\":\"...\"}}]}}"
    );
    let user = serialize_context(graph, &[]);
    (system, user)
}

pub fn chat_system(graph: &MapGraph, focus: &[String]) -> String {
    format!(
        "{SHARED_STANCE}\n\nThe user is chatting about the current map. Use it as context. When \
         you propose claims the user could add, state them as clear standalone sentences so they \
         can be dragged onto the canvas.\n\n{}",
        serialize_context(graph, focus)
    )
}
