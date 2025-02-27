# ADR-0025: Mermaid Diagram Support for MkDocs

## Status

Proposed

## Date

2025-02-27

## Context

Floxide is a directed graph workflow system, and visualizing these workflows is essential for documentation and understanding. Mermaid is a JavaScript-based diagramming tool that allows creating diagrams using Markdown-like syntax, making it ideal for our documentation needs.

While our MkDocs configuration already includes some Mermaid support, we need to ensure it's properly configured and working correctly across all documentation pages. This includes:

1. Proper rendering of Mermaid diagrams in the MkDocs site
2. Consistent styling that works with our terminal theme
3. Support for all diagram types needed for workflow visualization (flowcharts, sequence diagrams, class diagrams)
4. Proper deployment of the MkDocs site with Mermaid support

## Decision

We will enhance our MkDocs configuration to fully support Mermaid diagrams by:

1. Ensuring the proper Mermaid JavaScript library is included
2. Configuring the PyMdown Extensions for custom fences to properly render Mermaid diagrams
3. Adding initialization code to ensure Mermaid works with our terminal theme
4. Creating a GitHub workflow to build and deploy the MkDocs documentation site
5. Providing comprehensive examples in our documentation

## Consequences

### Advantages

- Improved visualization of workflows and architectural concepts
- Better documentation through visual representation
- Easier understanding of complex workflow patterns
- Consistent diagram styling across the documentation

### Disadvantages

- Additional JavaScript dependency
- Potential rendering issues with certain diagram types
- Need for ongoing maintenance of diagrams as the codebase evolves

## Implementation Details

1. Update `mkdocs.yml` to include:
   - The latest Mermaid JavaScript library
   - Proper configuration of PyMdown Extensions for Mermaid fences
   - Mermaid initialization code to work with the terminal theme

2. Create a GitHub workflow for building and deploying the MkDocs site

3. Update the existing Mermaid diagrams guide with additional examples specific to Floxide

## Alternatives Considered

### Using static images instead of Mermaid

Static images would be simpler but would make the documentation harder to maintain as diagrams would need to be regenerated whenever changes are made.

### Using a different diagramming tool

Other diagramming tools like PlantUML were considered, but Mermaid has better integration with Markdown and MkDocs, and its syntax is more intuitive for our needs.

## Related ADRs

- ADR-0024: MkDocs Documentation Setup with Terminal Theme

## References

- [Mermaid Official Documentation](https://mermaid-js.github.io/mermaid/)
- [PyMdown Extensions Documentation](https://facelessuser.github.io/pymdown-extensions/)
- [MkDocs Documentation](https://www.mkdocs.org/) 