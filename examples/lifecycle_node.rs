use async_trait::async_trait;
use floxide_core::{
    lifecycle::LifecycleNodeAdapter, DefaultAction, FloxideError, LifecycleNode, NodeId, Workflow,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

/// A simple context for our workflow
#[derive(Debug, Clone)]
struct TextProcessingContext {
    input: String,
    word_count: Option<usize>,
    character_count: Option<usize>,
    uppercase: Option<String>,
    notes: Vec<String>,
}

impl TextProcessingContext {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            word_count: None,
            character_count: None,
            uppercase: None,
            notes: Vec::new(),
        }
    }

    fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }
}

/// A node that counts words and characters in text
struct TextAnalysisNode {
    id: NodeId,
}

impl TextAnalysisNode {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

/// Analysis results from the preparation phase
#[derive(Debug, Clone)]
struct AnalysisPrep {
    text: String,
    _is_valid: bool,
}

/// Analysis results from the execution phase
#[derive(Debug, Clone)]
struct AnalysisResult {
    word_count: usize,
    character_count: usize,
}

#[async_trait]
impl LifecycleNode<TextProcessingContext, DefaultAction> for TextAnalysisNode {
    type PrepOutput = AnalysisPrep;
    type ExecOutput = AnalysisResult;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

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

    async fn post(
        &self,
        _prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut TextProcessingContext,
    ) -> Result<DefaultAction, FloxideError> {
        info!("Post-processing analysis results");

        // Update the context with the results
        ctx.word_count = Some(exec_result.word_count);
        ctx.character_count = Some(exec_result.character_count);

        ctx.add_note(&format!(
            "Analysis complete: {} words, {} characters",
            exec_result.word_count, exec_result.character_count
        ));

        // Determine the next action based on word count
        if exec_result.word_count > 10 {
            ctx.add_note("Text is lengthy, routing to uppercase conversion");
            Ok(DefaultAction::Next)
        } else {
            ctx.add_note("Text is short, completing workflow");
            Ok(DefaultAction::Next)
        }
    }
}

/// A node that converts text to uppercase
struct UppercaseNode {
    id: NodeId,
}

impl UppercaseNode {
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

    async fn prep(
        &self,
        ctx: &mut TextProcessingContext,
    ) -> Result<Self::PrepOutput, FloxideError> {
        info!("Preparing to convert text to uppercase");
        ctx.add_note("Preparing uppercase conversion");
        Ok(ctx.input.clone())
    }

    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError> {
        info!("Converting text to uppercase");
        let uppercase = prep_result.to_uppercase();
        Ok(uppercase)
    }

    async fn post(
        &self,
        _prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut TextProcessingContext,
    ) -> Result<DefaultAction, FloxideError> {
        info!("Uppercase conversion complete");
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
