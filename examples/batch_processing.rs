// Batch Processing Pattern: Parallel Data Processing with Controlled Concurrency
//
// This example demonstrates how to implement batch processing in the Flow Framework.
// Batch processing allows for efficient parallel processing of multiple items while
// maintaining control over resource usage and execution flow.
//
// Key concepts demonstrated:
// 1. Parallel processing with controlled concurrency
// 2. Resource management and optimization
// 3. Batch context for maintaining state across items
// 4. Error handling and statistics tracking
// 5. Progress monitoring and reporting
//
// The example processes a collection of images by:
// - Resizing them to 80% of their original dimensions
// - Converting them to WebP format
// - Tracking success/failure statistics
//
// This pattern is particularly useful for:
// - Processing large datasets efficiently
// - ETL (Extract, Transform, Load) operations
// - Media processing and conversion
// - Any scenario requiring parallel execution with resource constraints
//
// This example is designed in accordance with:
// - ADR-0012: Batch Processing Pattern
// - ADR-0016: Concurrency Control Strategy

use async_trait::async_trait;
use floxide_core::{
    batch::BatchContext, error::FloxideError, DefaultAction, Node, NodeId, NodeOutcome,
};
use rand::{Rng, rngs::StdRng, SeedableRng};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::fmt;
use uuid::Uuid;

/// Represents an image with various properties
///
/// This struct models an image with its metadata and content,
/// serving as the basic unit of work in our batch processing example.
#[derive(Debug, Clone)]
struct Image {
    /// Unique identifier for the image
    id: String,
    /// Filename or title of the image
    name: String,
    /// Width of the image in pixels
    width: u32,
    /// Height of the image in pixels
    height: u32,
    /// File format of the image (e.g., "jpg", "png", "webp")
    format: String,
    /// Simulated image data (in a real application, this would be binary data)
    data: String,
    /// Time taken to process this image in milliseconds
    processing_time_ms: u64,
}

impl Image {
    /// Creates a new image with the given properties
    ///
    /// Initializes an image with a random ID and simulated processing time.
    fn _new(name: &str, width: u32, height: u32, format: &str) -> Self {
        // Simulate random processing time for this image
        let mut rng = StdRng::from_entropy();
        let processing_time_ms = rng.gen_range(100..1000);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            width,
            height,
            format: format.to_string(),
            data: format!("Original image data for {}", name),
            processing_time_ms,
        }
    }

    /// Resizes the image to the specified dimensions
    ///
    /// Simulates the time it takes to resize an image.
    async fn _resize(&self, new_width: u32, new_height: u32) -> Result<Image, FloxideError> {
        // Simulate the time it takes to resize an image
        sleep(Duration::from_millis(self.processing_time_ms)).await;

        let mut resized = self.clone();
        resized.width = new_width;
        resized.height = new_height;
        resized.data = format!("Resized: {}", self.data);
        info!(
            "Resized image {} to {}x{}",
            self.name, new_width, new_height
        );

        Ok(resized)
    }

    /// Converts the image to the specified format
    ///
    /// Simulates the time it takes to convert an image.
    async fn _convert_format(&self, new_format: &str) -> Result<Image, FloxideError> {
        // Simulate the time it takes to convert an image
        sleep(Duration::from_millis(self.processing_time_ms / 2)).await;

        let mut converted = self.clone();
        converted.format = new_format.to_string();
        converted.data = format!("Converted to {}: {}", new_format, self.data);
        info!("Converted image {} to {}", self.name, new_format);

        Ok(converted)
    }
}

/// Context for batch processing of images
///
/// This context implements the BatchContext trait, which is the core abstraction
/// for batch processing in Floxide. It manages:
/// - The collection of items to be processed (images)
/// - The current item being processed (used in item-specific contexts)
/// - Statistics about processing success/failure
/// - Any additional batch-specific state
#[derive(Debug, Clone)]
struct ImageBatchContext {
    /// Collection of images to be processed in the batch
    images: Vec<Image>,
    /// The current image being processed (used in item-specific contexts)
    current_image: Option<Image>,
    /// Number of successfully processed images
    processed_count: usize,
    /// Number of images that failed processing
    failed_count: usize,
    /// Target format for image conversion
    target_format: String,
    /// Statistics collected during batch processing
    stats: HashMap<String, usize>,
}

impl ImageBatchContext {
    /// Creates a new batch context with the given images and target format
    ///
    /// Initializes the batch context with the specified images and target format.
    fn _new(images: Vec<Image>, target_format: &str) -> Self {
        Self {
            images,
            current_image: None,
            processed_count: 0,
            failed_count: 0,
            target_format: target_format.to_string(),
            stats: HashMap::new(),
        }
    }

    /// Increments a named statistic counter
    ///
    /// Updates the batch context with the specified statistic.
    fn add_stat(&mut self, key: &str) {
        *self.stats.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Prints the batch processing statistics
    ///
    /// Displays the collected statistics for the batch processing operation.
    fn _print_stats(&self) {
        info!("Batch processing statistics:");
        for (key, value) in &self.stats {
            info!("  {}: {}", key, value);
        }
    }

    /// Updates the batch context with the results of processing multiple items
    ///
    /// This method is called after a batch of items has been processed to
    /// update the overall batch context with the results. It's where you
    /// would aggregate statistics, handle errors, or update any batch-level state.
    fn update_with_results(
        &mut self,
        results: &[Result<Image, FloxideError>],
    ) -> Result<(), FloxideError> {
        // Count successes and failures
        self.processed_count = results.iter().filter(|r| r.is_ok()).count();
        self.failed_count = results.iter().filter(|r| r.is_err()).count();

        // Update statistics for each result
        for result in results {
            match result {
                Ok(_) => self.add_stat("success"),
                Err(_) => self.add_stat("failure"),
            }
        }

        Ok(())
    }
}

/// Implementation of the BatchContext trait for image processing
///
/// The BatchContext trait is the foundation of batch processing in Floxide.
/// It defines three key methods:
/// 1. get_batch_items: Returns all items to be processed
/// 2. create_item_context: Creates a context for a single item
/// 3. update_with_results: Updates the batch context with processing results
impl BatchContext<Image> for ImageBatchContext {
    /// Returns all images to be processed in this batch
    ///
    /// This method is called by the batch processing system to get the
    /// complete list of items that need to be processed.
    fn get_batch_items(&self) -> Result<Vec<Image>, FloxideError> {
        Ok(self.images.clone())
    }

    /// Creates a context for processing a single image
    ///
    /// This method is called for each item in the batch to create a context
    /// that will be used when processing just that item. It allows the
    /// batch processor to handle each item in isolation.
    fn create_item_context(&self, item: Image) -> Result<Self, FloxideError> {
        // Create a new context for a single item
        let mut ctx = self.clone();
        ctx.images = Vec::new();  // Clear the batch items
        ctx.current_image = Some(item);  // Set the current item
        Ok(ctx)
    }

    /// Updates the batch context with the results of processing multiple items
    ///
    /// This method is called after a batch of items has been processed to
    /// update the overall batch context with the results.
    fn update_with_results(&mut self, results: &[Result<Image, FloxideError>]) -> Result<(), FloxideError> {
        // Count successes and failures
        self.processed_count += results.iter().filter(|r| r.is_ok()).count();
        self.failed_count += results.iter().filter(|r| r.is_err()).count();

        // Update statistics for each result
        for result in results {
            match result {
                Ok(_) => self.add_stat("success"),
                Err(_) => self.add_stat("failure"),
            }
        }

        Ok(())
    }
}

/// A simple image processor node that implements the Node trait
///
/// This node processes a single image by:
/// - Resizing it to 80% of its original dimensions
/// - Converting it to WebP format
/// - Occasionally failing randomly (15% chance) to demonstrate error handling
struct SimpleImageProcessor {
    id: NodeId,
}

impl SimpleImageProcessor {
    /// Creates a new image processor node with the given ID
    ///
    /// Initializes the node with the specified ID.
    fn new(id: &str) -> Self {
        SimpleImageProcessor { id: id.to_string() }
    }
}

#[async_trait]
impl Node<ImageBatchContext, DefaultAction> for SimpleImageProcessor
where
    Self: Send + Sync,
    ImageBatchContext: Send,
{
    type Output = Image;

    /// Returns the ID of the node
    ///
    /// This method is used to identify the node in the workflow.
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Processes the image using the provided context
    ///
    /// This method is called to process a single image using the node.
    async fn process(
        &self,
        ctx: &mut ImageBatchContext,
    ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
        // Get the current image from the context
        let image = match &ctx.current_image {
            Some(img) => img.clone(),
            None => {
                return Err(FloxideError::node_execution(
                    "SimpleImageProcessor",
                    "No image found in context",
                ))
            }
        };

        info!(
            "Processing image: {} ({}x{}, {})",
            image.name, image.width, image.height, image.format
        );

        // Resize the image to 80% of its original size
        let new_width = (image.width as f32 * 0.8) as u32;
        let new_height = (image.height as f32 * 0.8) as u32;
        let resized = image._resize(new_width, new_height).await?;

        // Simulate a random failure (15% chance) - using StdRng which is Send
        let mut rng = StdRng::from_entropy();
        if rng.gen_range(0..100) < 15 {
            ctx.add_stat("failed");
            return Err(FloxideError::node_execution(
                "SimpleImageProcessor",
                &format!("Failed to process image: {}", image.id),
            ));
        }

        // Convert the image to WebP format
        let converted = resized._convert_format(&ctx.target_format).await?;

        info!(
            "Successfully processed image: {} -> {}x{}, {}",
            converted.name, converted.width, converted.height, converted.format
        );

        ctx.add_stat("processed");
        Ok(NodeOutcome::Success(converted))
    }
}

/// Process a single image using the SimpleImageProcessor
///
/// This function demonstrates how to process a single item using a node.
/// In a batch processing scenario, this function would be called for each
/// item in the batch, potentially in parallel.
async fn process_image(image: Image) -> Result<Image, FloxideError> {
    // Create a context for this single image
    let mut ctx = ImageBatchContext {
        images: vec![],
        current_image: Some(image),
        processed_count: 0,
        failed_count: 0,
        target_format: "webp".to_string(),
        stats: HashMap::new(),
    };

    // Create and use the processor node
    let processor = SimpleImageProcessor::new("image-processor");
    let result = processor.process(&mut ctx).await?;

    match result {
        NodeOutcome::Success(processed_image) => Ok(processed_image),
        _ => Err(FloxideError::node_execution(
            "process_image",
            "Unexpected node outcome",
        )),
    }
}

/// Process images in parallel with a given parallelism limit
///
/// This function demonstrates a key aspect of batch processing: controlled parallelism.
/// It processes multiple items concurrently while limiting the maximum number of
/// simultaneous operations to avoid overwhelming system resources.
///
/// Key components:
/// - StreamExt from futures: Provides methods for working with asynchronous streams
/// - Semaphore from tokio: Controls the maximum number of concurrent operations
/// - buffer_unordered: Processes items concurrently while maintaining the parallelism limit
///
/// The function returns a vector of results, preserving the success or failure
/// status of each item's processing.
async fn process_batch(
    images: Vec<Image>,
    parallelism: usize,
) -> Vec<Result<Image, FloxideError>> {
    use futures::stream::{self, StreamExt};
    use tokio::sync::Semaphore;
    use std::sync::Arc;

    // Create a semaphore to limit concurrency
    let semaphore = Arc::new(Semaphore::new(parallelism));

    // Process all images concurrently with controlled parallelism
    let results = stream::iter(images)
        .map(|image| {
            let sem = semaphore.clone();
            async move {
                // Acquire a permit from the semaphore
                let _permit = sem.acquire().await.unwrap();

                // Process the image
                let result = process_image(image).await;

                // The permit is automatically released when it goes out of scope
                result
            }
        })
        .buffer_unordered(parallelism) // Process up to `parallelism` items concurrently
        .collect::<Vec<_>>()
        .await;

    results
}

/// Main function to demonstrate batch processing
///
/// This function:
/// 1. Sets up logging for the example
/// 2. Creates a batch of sample images
/// 3. Processes the batch with controlled parallelism
/// 4. Collects and displays statistics about the processing
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing subscriber for logging
    fmt().with_max_level(Level::INFO).init();

    info!("Starting batch processing example");

    // Create a batch of sample images
    let mut images = Vec::new();
    let mut rng = StdRng::from_entropy();
    for i in 1..=20 {
        let name = format!("image_{}.jpg", i);
        let width = rng.gen_range(800..2000);
        let height = rng.gen_range(600..1500);
        images.push(Image::_new(&name, width, height, "jpg"));
    }

    info!("Created {} sample images", images.len());

    // Process the batch with a parallelism of 4
    let results = process_batch(images.clone(), 4).await;

    // Create a batch context to track statistics
    let mut batch_ctx = ImageBatchContext::_new(images, "webp");

    // Update the batch context with the results
    batch_ctx.update_with_results(&results)?;

    // Print the statistics
    batch_ctx._print_stats();

    info!("Batch processing example completed");

    Ok(())
}
