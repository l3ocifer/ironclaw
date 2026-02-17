# Weekly Review Template

Schedule for Sunday (or your preferred review day).

```markdown
## Weekly Review — Week of {{date}}

### Wins this week
-

### What didn't work?
-

### Goal progress
- **[Goal 1]:** [Status, trajectory]
- **[Goal 2]:** [Status, trajectory]
- **[Goal 3]:** [Status, trajectory]

### Project status
- **Active:** [list]
- **Blocked:** [list and why]
- **Completed:** [list]

### Next week's focus (top 3)
1.
2.
3.

### Ideas to explore
-

### Notes for the agent
- [Research, prep, or recurring tasks for next week]
```

## Cron Configuration

```json
{
  "name": "weekly-review",
  "cron": "0 10 * * 0",
  "session": "isolated",
  "message": "Prepare the weekly review. Summarize this week's daily logs, project progress, and goal status. Write to daily log and alert me.",
  "announce": true,
  "channel": "telegram"
}
```
