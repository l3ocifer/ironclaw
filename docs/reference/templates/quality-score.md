# Quality Score Template

Grade each domain and layer of your project. Review weekly or on each major change.
This gives the agent (and humans) a legible map of where quality is strong and where it's soft.

```markdown
# Quality Score — [Project Name]

**Last updated:** YYYY-MM-DD
**Overall:** B+

## Domain Scores

| Domain | Tests | Docs | Reliability | UX | Grade | Notes |
|--------|-------|------|-------------|-----|-------|-------|
| Auth | A | B | A | B+ | A- | Solid coverage, docs need examples |
| Billing | B | C | B+ | B | B | Missing edge case tests |
| Dashboard | C | B | B | A | B | Needs integration tests |
| API | A | A | A | A | A | Reference quality |

## Layer Scores

| Layer | Coverage | Lint Clean | Perf Budget | Grade |
|-------|----------|------------|-------------|-------|
| Types/Schema | A | A | — | A |
| Services | B | A | B | B+ |
| UI Components | C | B | B | C+ |
| Infrastructure | B | A | A | A- |

## Tracked Gaps
- [ ] Dashboard integration tests (C → B)
- [ ] Billing edge cases (B → A)
- [ ] UI component test coverage (C → B)

## Grading Scale
- **A:** Production-ready, well-tested, documented, handles edge cases
- **B:** Functional, mostly tested, some gaps
- **C:** Works but fragile, under-tested, or under-documented
- **D:** Known broken or missing critical coverage
```

Update grades when shipping changes. The agent should reference this when prioritizing
cleanup work and flag regressions during heartbeat or review cycles.
