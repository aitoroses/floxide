use floxide_core::{node::Node, transition::Transition, Workflow};
use floxide_macros::{node, workflow};

// A simple stub node that just passes through a value or fails once for retry demo
// Define a printing node that finishes the workflow
node! {
    pub struct StubNode { name: &'static str };
    context = ();
    input = ();
    output = ();
    |_ctx, _x| {
        Ok(Transition::Next(()))
    }
}

impl StubNode {
    fn new(name: &'static str) -> Self {
        Self { name }
    }
}

workflow! {
    pub struct ExampleWorkflow {
        #[retry = retry_policy]
        retry_policy: StubNode,
        start: StubNode,
        task_a: StubNode,
        task_b: StubNode,
        fallback_a: StubNode,
        task_c: StubNode,
        task_d: StubNode,
        join_point: StubNode,
        end_node: StubNode,
    }
    start = start;
    context = ();
    edges {
        // linear path
        start => { [ task_a ] };
        task_a => { [ task_b ] };
        task_b => { [ end_node ] };

        // retry on TaskB
        task_b on_failure => { [ task_a ] };

        // fallback on TaskA
        task_a on_failure => { [ fallback_a ] };

        // composite branch example
        task_b => {
            // pretend TaskB can output two variants
            // but using the same stub values
            StubNode::Branch(x) => [ task_c ];
            StubNode::Branch2(x) => [ task_d ];
        };

        // parallel then join
        task_c => { [ join_point ] };
        task_d => { [ join_point ] };
        join_point => { [ end_node ] };

        // fallback on TaskC
        fallback_a  => { [ end_node ] };

        // terminal
        end_node => {};
    }
}

fn main() {
    // Build the workflow instance
    let wf = ExampleWorkflow {
        retry_policy: StubNode::new("RetryPolicy"),
        start: StubNode::new("Start"),
        task_a: StubNode::new("TaskA"),
        task_b: StubNode::new("TaskB"),
        fallback_a: StubNode::new("FallbackA"),
        task_c: StubNode::new("TaskC"),
        task_d: StubNode::new("TaskD"),
        join_point: StubNode::new("JoinPoint"),
        end_node: StubNode::new("End"),
    };

    // Generate and print the DOT graph
    let dot = wf.to_dot();
    println!("{}", dot);
}
