// Lifecycle Node Pattern: A Multi-Phase Node Execution Example
//
// This example demonstrates how to implement and use the Lifecycle Node pattern
// in the Flow Framework. The Lifecycle Node pattern breaks down node execution
// into three distinct phases: preparation, execution, and post-processing.
//
// Key concepts demonstrated:
// 1. Multi-phase node execution with distinct responsibilities
// 2. Separation of concerns between validation, processing, and result handling
// 3. Type-safe data flow between execution phases
// 4. Error handling at different stages of execution
// 5. Context updates at appropriate lifecycle phases
//
// The example implements a text processing workflow with two nodes:
// - TextAnalysisNode: Analyzes text to count words and characters
// - UppercaseNode: Converts text to uppercase
//
// Each node follows the lifecycle pattern with three phases:
// 1. prep: Validates inputs and prepares for execution
// 2. exec: Performs the main processing logic
// 3. post: Updates the context with results and determines next action
//
// This pattern is particularly useful for complex processing tasks where
// you want clear separation between input validation, core processing logic,
// and result handling.
//
// This example is designed in accordance with:
// - ADR-0012: Lifecycle Node Pattern
// - ADR-0015: Multi-Phase Execution Model

use async_trait::async_trait;
use floxide_core::{
    lifecycle::LifecycleNodeAdapter, DefaultAction, FloxideError, LifecycleNode, NodeId, Workflow,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

/// A simple context for our workflow
///
/// This context stores the input text, analysis results, and processing notes.
/// It's passed between nodes and updated as the workflow progresses.
#[derive(Debug, Clone)]
struct TextProcessingContext {
    /// Original input text to be processed
    input: String,
    /// Number of words in the input text (set by TextAnalysisNode)
    word_count: Option<usize>,
    /// Number of characters in the input text (set by TextAnalysisNode)
    character_count: Option<usize>,
    /// Uppercase version of the input text (set by UppercaseNode)
    uppercase: Option<String>,
    /// Processing notes and logs from each step
    notes: Vec<String>,
}

impl TextProcessingContext {
    /// Creates a new context with the given input text
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            word_count: None,
            character_count: None,
            uppercase: None,
            notes: Vec::new(),
        }
    }

    /// Adds a processing note to the context
    ///
    /// This is used to track the progress and decisions made during workflow execution.
    fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }
}

/// A node that counts words and characters in text
///
/// This node demonstrates the Lifecycle Node pattern by implementing
/// the LifecycleNode trait with three distinct phases:
/// - prep: Validates that the input text is not empty
/// - exec: Counts words and characters in the text
/// - post: Updates the context with the analysis results
struct TextAnalysisNode {
    id: NodeId,
}

impl TextAnalysisNode {
    /// Creates a new TextAnalysisNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

/// Analysis results from the preparation phase
///
/// This struct is returned by the prep phase and passed to the exec phase.
/// It contains the validated text and any metadata needed for execution.
#[derive(Debug, Clone)]
struct AnalysisPrep {
    /// The validated text to be analyzed
    text: String,
    /// Flag indicating whether the text is valid
    _is_valid: bool,
}

/// Analysis results from the execution phase
///
/// This struct is returned by the exec phase and passed to the post phase.
/// It contains the analysis results that will be stored in the context.
#[derive(Debug, Clone)]
struct AnalysisResult {
    /// Number of words in the text
    word_count: usize,
    /// Number of characters in the text
    character_count: usize,
}

#[async_trait]
impl LifecycleNode<TextProcessingContext, DefaultAction> for TextAnalysisNode {
    type PrepOutput = AnalysisPrep;
    type ExecOutput = AnalysisResult;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Preparation phase: Validates the input text
    ///
    /// This phase:
    /// 1. Checks if the input text is empty
    /// 2. Returns an error if validation fails
    /// 3. Otherwise, returns the validated text for processing
    async fn prep(
        &self,
        ctx: &mut TextProcessingContext,
    ) -> Result<Self::PrepOutput, FloxideError> {
        info!("Preparing to analyze text: '{}'", ctx.input);

        // Validate the input
        let is_valid = !ctx.input.trim().is_empty();

        if !is_valid {
            ctx.add_note("Validation failed: Input text is empty");
            return Err(FloxideError::node_execution(
                self.id(),
                "Input text cannot be empty",
            ));
        }

        ctx.add_note("Text validation successful");

        Ok(AnalysisPrep {
            text: ctx.input.clone(),
            _is_valid: is_valid,
        })
    }

    /// Execution phase: Counts words and characters
    ///
    /// This phase takes the validated text from the prep phase
    /// and counts the number of words and characters.
    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        info!("Executing text analysis");

        // Count words and characters
        let word_count = prep_result.text.split_whitespace().count();
        let character_count = prep_result.text.chars().count();

        info!(
            "Analysis complete: {} words, {} characters",
            word_count, character_count
        );

        Ok(AnalysisResult {
            word_count,
            character_count,
        })
    }

    /// Post-processing phase: Updates the context with results
    ///
    /// This phase updates the context with the analysis results
    /// and determines the next action to take.
    async fn post(
        &self,
        _prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut TextProcessingContext,
    ) -> Result<DefaultAction, FloxideError> {
        info!("Post-processing analysis results");

        // Update the context with the analysis results
        ctx.word_count = Some(exec_result.word_count);
        ctx.character_count = Some(exec_result.character_count);

        // Add notes about the analysis
        ctx.add_note(&format!(
            "Text analysis complete: {} words, {} characters",
            exec_result.word_count, exec_result.character_count
        ));

        // Determine the next action based on the results
        if exec_result.word_count > 0 {
            info!("Analysis successful, proceeding to next step");
            Ok(DefaultAction::Next)
        } else {
            info!("Analysis found no words, workflow will end");
            Ok(DefaultAction::Next)
        }
    }
}

/// A node that converts text to uppercase
///
/// This node demonstrates another implementation of the Lifecycle Node pattern
/// with simpler input/output types. It converts the input text to uppercase.
struct UppercaseNode {
    id: NodeId,
}

impl UppercaseNode {
    /// Creates a new UppercaseNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl LifecycleNode<TextProcessingContext, DefaultAction> for UppercaseNode {
    type PrepOutput = String;
    type ExecOutput = String;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Preparation phase: Extracts the input text
    ///
    /// This phase simply extracts the input text from the context
    /// and passes it to the execution phase.
    async fn prep(
        &self,
        ctx: &mut TextProcessingContext,
    ) -> Result<Self::PrepOutput, FloxideError> {
        info!("Preparing to convert text to uppercase");
        ctx.add_note("Starting uppercase conversion");
        Ok(ctx.input.clone())
    }

    /// Execution phase: Converts text to uppercase
    ///
    /// This phase takes the input text and converts it to uppercase.
    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        info!("Converting text to uppercase");
        Ok(prep_result.to_uppercase())
    }

    /// Post-processing phase: Updates the context with uppercase text
    ///
    /// This phase updates the context with the uppercase text
    /// and returns the next action to take.
    async fn post(
        &self,
        _prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut TextProcessingContext,
    ) -> Result<DefaultAction, FloxideError> {
        info!("Post-processing uppercase conversion");
        ctx.uppercase = Some(exec_result);
        ctx.add_note("Uppercase conversion complete");
        Ok(DefaultAction::Next)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create nodes
    let analysis_node = TextAnalysisNode::new();
    let uppercase_node = UppercaseNode::new();

    // Adapt the nodes to make them implement the Node trait
    let analysis_adapter = LifecycleNodeAdapter::new(analysis_node);
    let uppercase_adapter = LifecycleNodeAdapter::new(uppercase_node);

    // Create workflows for each node
    let analysis_workflow = Workflow::new(analysis_adapter);
    let uppercase_workflow = Workflow::new(uppercase_adapter);

    // Process short text with first workflow
    info!("\n--- Processing short text with analysis workflow ---");
    let mut short_context = TextProcessingContext::new("Hello!");

    if let Err(err) = analysis_workflow.execute(&mut short_context).await {
        info!("Workflow error: {}", err);
    }

    // If the analysis completes successfully, run the uppercase workflow
    if short_context.word_count.is_some() {
        info!("\n--- Processing with uppercase workflow ---");
        if let Err(err) = uppercase_workflow.execute(&mut short_context).await {
            info!("Uppercase workflow error: {}", err);
        }
    }

    // Display results
    info!("Short text results:");
    info!("Word count: {:?}", short_context.word_count);
    info!("Character count: {:?}", short_context.character_count);
    info!("Uppercase: {:?}", short_context.uppercase);
    info!("Notes:");
    for (i, note) in short_context.notes.iter().enumerate() {
        info!("  {}. {}", i + 1, note);
    }

    // Process a longer text
    info!("\n--- Processing longer text with analysis workflow ---");
    let mut long_context = TextProcessingContext::new(
        "This is a longer text that will be processed through both nodes in the workflow.",
    );

    if let Err(err) = analysis_workflow.execute(&mut long_context).await {
        info!("Workflow error: {}", err);
    }

    // If the analysis completes successfully, run the uppercase workflow
    if long_context.word_count.is_some() {
        info!("\n--- Processing with uppercase workflow ---");
        if let Err(err) = uppercase_workflow.execute(&mut long_context).await {
            info!("Uppercase workflow error: {}", err);
        }
    }

    info!("Long text results:");
    info!("Word count: {:?}", long_context.word_count);
    info!("Character count: {:?}", long_context.character_count);
    info!("Uppercase: {:?}", long_context.uppercase);
    info!("Notes:");
    for (i, note) in long_context.notes.iter().enumerate() {
        info!("  {}. {}", i + 1, note);
    }

    // Try with empty text (should fail validation)
    info!("\n--- Processing empty text ---");
    let mut empty_context = TextProcessingContext::new("");

    if let Err(err) = analysis_workflow.execute(&mut empty_context).await {
        info!("Workflow error (expected): {}", err);
    }

    info!("Empty text results:");
    info!("Word count: {:?}", empty_context.word_count);
    info!("Character count: {:?}", empty_context.character_count);
    info!("Uppercase: {:?}", empty_context.uppercase);
    info!("Notes:");
    for (i, note) in empty_context.notes.iter().enumerate() {
        info!("  {}. {}", i + 1, note);
    }

    Ok(())
}
