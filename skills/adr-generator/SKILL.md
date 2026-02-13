---
name: adr-generator
description: Creates Architecture Decision Records for significant technical decisions. Use when making architectural choices or documenting design decisions.
---

# Architecture Decision Record (ADR) Generator

Creates standardized ADR documents for architectural and technical decisions.

## Activation

Use when:
- User says "create an ADR"
- Major architectural decision is being made
- New technology or pattern is being adopted
- Design pattern choice needs documentation
- User asks to "document this decision"

## Instructions

1. Identify the decision context and problem
2. List alternatives considered
3. Document the chosen approach
4. Explain consequences (positive and negative)
5. Add implementation notes if applicable

## ADR Template

```markdown
# ADR-{number}: {Title}

**Date:** {YYYY-MM-DD}
**Status:** {Proposed | Accepted | Deprecated | Superseded | Rejected}
**Deciders:** {List of people involved}

## Context

### Problem Statement
What issue or need triggered this decision?

### Constraints
- Technical constraints
- Business constraints
- Timeline constraints
- Resource constraints

### Assumptions
- Key assumptions made
- Dependencies on other systems/decisions

## Decision

### Chosen Approach
Detailed description of the selected solution.

### Rationale
Why this approach was chosen over alternatives.

## Alternatives Considered

### Alternative A: {Name}
- **Description:** Brief overview
- **Pros:** Benefits
- **Cons:** Drawbacks
- **Why rejected:** Specific reasons

### Alternative B: {Name}
- **Description:** Brief overview
- **Pros:** Benefits
- **Cons:** Drawbacks
- **Why rejected:** Specific reasons

## Consequences

### Positive
- Benefit 1: Explanation
- Benefit 2: Explanation
- Benefit 3: Explanation

### Negative
- Trade-off 1: Explanation
- Trade-off 2: Explanation
- Risk 1: Mitigation strategy

### Neutral
- Change 1: Impact
- Change 2: Impact

## Implementation

### Steps
1. Step 1
2. Step 2
3. Step 3

### Timeline
- Phase 1: Description
- Phase 2: Description

### Required Resources
- Team resources
- Infrastructure needs
- Dependencies

## Related Decisions

- ADR-XXX: Related decision
- ADR-YYY: Superseded by this

## References

- [Link 1](url): Description
- [Link 2](url): Description
```

## Example ADR

```markdown
# ADR-003: Use PostgreSQL for Primary Database

**Date:** 2024-11-14
**Status:** Accepted
**Deciders:** Engineering Team, CTO

## Context

### Problem Statement
We need to select a primary database for our multi-tenant SaaS application that will handle:
- 10M+ records initially
- Complex relational queries
- ACID compliance requirements
- Real-time analytics

### Constraints
- Must support multi-tenancy (row-level security)
- Budget: < $500/month for initial scale
- Team has limited NoSQL experience
- Must integrate with existing AWS infrastructure

## Decision

### Chosen Approach
PostgreSQL 15 with AWS RDS

### Rationale
- Native JSON support for flexible schema
- Excellent ACID compliance
- Row-level security for multi-tenancy
- Team has strong SQL experience
- AWS RDS provides managed service

## Alternatives Considered

### Alternative A: MongoDB
- **Pros:** Schema flexibility, horizontal scaling
- **Cons:** Eventual consistency, team learning curve
- **Why rejected:** ACID requirements and team expertise

### Alternative B: MySQL
- **Pros:** Team familiarity, proven scale
- **Cons:** Weaker JSON support, less advanced features
- **Why rejected:** Need for JSON and advanced indexing

## Consequences

### Positive
- Strong consistency guarantees
- No additional training needed
- Better query performance for complex joins
- Built-in full-text search

### Negative
- Vertical scaling limits
- Higher cost at massive scale
- Less flexible schema changes

## Implementation

### Steps
1. Provision RDS instance in production VPC
2. Set up automated backups (daily, 30-day retention)
3. Configure connection pooling (PgBouncer)
4. Implement row-level security policies
5. Set up monitoring and alerting

### Timeline
- Week 1: Infrastructure setup
- Week 2: Schema migration
- Week 3: Testing and validation
- Week 4: Production cutover

## References

- [PostgreSQL JSON docs](https://www.postgresql.org/docs/current/datatype-json.html)
- [AWS RDS Best Practices](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/CHAP_BestPractices.html)
```

## Best Practices

- **Keep it concise:** ADRs should be readable in 5-10 minutes
- **Update status:** Mark as Superseded when decision changes
- **Link decisions:** Reference related ADRs
- **Include dates:** Track when decisions were made
- **Be honest:** Document real trade-offs, not idealized versions
- **Review regularly:** Revisit decisions quarterly

