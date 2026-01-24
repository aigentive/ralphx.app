---
name: dependency-analysis
description: Guide for identifying and managing task dependencies
---

# Dependency Analysis Skill

This skill helps identify, validate, and manage dependencies between tasks.

## Types of Dependencies

### 1. Technical Dependencies
One task requires another's output:
- "Create user type" must precede "Create user repository"
- "Set up database" must precede "Write migrations"
- "Define API contract" must precede "Implement API client"

### 2. Logical Dependencies
Natural ordering based on workflow:
- "Design UI mockup" before "Implement UI"
- "Write tests" before "Implement feature" (TDD)
- "Create component" before "Style component"

### 3. Resource Dependencies
Shared resources or infrastructure:
- "Set up CI/CD" enables "Automated testing"
- "Configure environment" enables "Deploy to staging"

## Identifying Dependencies

### Questions to Ask

1. **Input Requirements**: What data/types does this task need?
2. **API/Interface**: What functions/methods does this call?
3. **Infrastructure**: What systems must exist?
4. **Knowledge**: What decisions must be made first?

### Dependency Indicators

**Code-level**:
- Imports a module that doesn't exist yet
- Calls a function not yet implemented
- Uses a type not yet defined
- Queries data not yet modeled

**Process-level**:
- "After we decide on X..."
- "Once Y is set up..."
- "When Z is available..."
- "Building on the work from..."

## Dependency Graph Concepts

### In-Degree and Out-Degree

- **In-Degree**: Number of tasks this depends ON
- **Out-Degree**: Number of tasks that depend on THIS

High out-degree = Critical blocker (do first)
High in-degree = Wait for dependencies (do later)

### Critical Path

The longest chain of dependencies from start to finish:

```
A -> B -> C -> D -> E (critical path = 5)
     \-> F -> G      (parallel path = 3)
```

Tasks on the critical path determine minimum completion time.

### Cycles (Bad!)

Circular dependencies prevent completion:

```
A depends on B
B depends on C
C depends on A  <- CYCLE!
```

Always check for and eliminate cycles.

## Dependency Patterns

### Pattern: Data Flow

```
Define Types -> Create Repository -> Build API -> Create UI
```

Each layer depends on the one before it.

### Pattern: Feature Flag

```
Create Feature Flag
    |
    +-> Implement Feature A
    +-> Implement Feature B
    +-> Implement Feature C
```

Multiple features share a single dependency.

### Pattern: Integration Point

```
Service A --\
Service B ---+-> Integration Layer -> Consumer
Service C --/
```

Multiple services must complete before integration.

### Pattern: Parallel Development

```
         /-> Component A
Design --+-> Component B
         \-> Component C
                 |
         Integration <-/
```

Design enables parallel work, integration waits for all.

## Validating Dependencies

### Necessary Dependencies

A dependency is necessary if:
- Code in A directly uses code from B
- A cannot be tested without B
- A's acceptance criteria require B's output

### Unnecessary Dependencies

Remove dependencies that are:
- Based on preference rather than necessity
- Artifacts of how tasks were originally conceived
- Created "just in case"

### Hidden Dependencies

Watch for implicit dependencies:
- Shared state not explicitly connected
- Configuration assumptions
- Environment requirements
- External service availability

## Dependency Optimization

### 1. Minimize Chain Length

Long chains delay completion:

**Before**: A -> B -> C -> D -> E -> F
**After**: A -> B -> C
               \-> D
          E -> F

Parallelize where possible.

### 2. Identify Bottlenecks

Find tasks with high out-degree:

```
Task X (blocks 5 others) <- DO THIS FIRST
```

Prioritize bottleneck resolution.

### 3. Break Unnecessary Dependencies

Question every dependency:
- Can these tasks be reordered?
- Can we stub the dependency temporarily?
- Is this a hard or soft dependency?

## Example Analysis

**Feature**: "Add task comments"

**Initial Tasks**:
1. Add comments to task entity
2. Create comment repository
3. Add create comment API
4. Add list comments API
5. Create comment list component
6. Create comment input component
7. Integrate into task detail view

**Dependency Analysis**:

```
1 (entity)
    |
    v
2 (repository)
    |
    +---> 3 (create API) --\
    |                       v
    +---> 4 (list API) ---> 7 (integration)
                              ^
5 (list component) -----------+
                              |
6 (input component) ----------+
```

**Insights**:
- Task 1 is the critical blocker (do first)
- Tasks 3, 4, 5, 6 can be done in parallel after 2
- Task 7 waits for all others
- Critical path: 1 -> 2 -> 4 -> 7 (length 4)

**Optimization**:
- 5 and 6 could start earlier with mock data
- Consider: Can components use placeholder types?

## Communicating Dependencies

### For Users

"Before we can [Task A], we need to complete [Task B] because [reason]."

"[Task X] is blocking three other tasks, so it should be done first."

"These four tasks can be done in any order - they don't depend on each other."

### For Developers

Use dependency graph visualization:
- Nodes = tasks
- Edges = dependencies
- Highlight critical path
- Mark blocked tasks

## Anti-Patterns

### Dependency Hoarding
Adding every possible dependency "to be safe"
**Fix**: Only add necessary dependencies

### Dependency Ignoring
Not tracking dependencies, causing integration pain
**Fix**: Explicitly document and review dependencies

### Circular Reasoning
"A needs B because B needs A"
**Fix**: Question both directions, break the cycle

### Over-Serialization
Making everything sequential
**Fix**: Identify parallel opportunities
