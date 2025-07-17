# Three Dimensions of Context

## Overview
Patina manages three orthogonal dimensions that compose to create a complete development environment:

## 1. LLM Choice (How we communicate)
- Claude, Gemini, Local, OpenAI
- Determines output format (CLAUDE.md, GEMINI.md, etc.)
- Determines available tools (MCP for Claude)
- Determines context limits and optimization strategies

## 2. Knowledge (What we know)
- **Core**: Universal principles (Unix philosophy, Rust patterns)
- **Topics**: Domain knowledge (blockchain, game-dev, web-services)
- **Projects**: Specific implementations and patterns
- This is the brain - pure, unchanging truth

## 3. Environment (Where we execute)
- **Development Environment**: Where you build (Mac + Dagger, Linux + Docker, etc.)
- **Deployment Environment**: Where it runs (Docker for apps, native for tools)
- Includes available tools, paths, and safety constraints

## Composition
```
Project Context = Knowledge × LLM × Environment
```

## Key Insights
- Knowledge is constant across all dimensions
- LLM choice affects presentation, not content
- Environment splits into dev vs deploy
- Most apps: AI-assisted development → Traditional deployment

## Examples
1. Dev: Mac + Claude + Dagger → Deploy: Docker
2. Dev: Linux + Gemini + Nix → Deploy: Docker  
3. Dev: Windows + Local + Docker → Deploy: Native binary (for tools)