# Monthly Review Template

Schedule for the 1st of each month.

```markdown
## Monthly Review — {{month}} {{year}}

### Last month summary
- **Major accomplishments:**
  -
- **Major challenges:**
  -

### Goal progress
- **[Goal 1]:** Current state → next milestone
- **[Goal 2]:** Current state → next milestone
- **[Goal 3]:** Current state → next milestone

### Keep / Stop / Start
- **Keep doing:**
  -
- **Stop doing:**
  -
- **Start doing:**
  -

### Projects
- **Active:** [list with status]
- **On hold:** [list with reason]
- **To archive:** [list]

### Next month's theme
-

### Notes for the agent
- [Monthly prep, recurring tasks, research queue]
```

## Cron Configuration

```json
{
  "name": "monthly-review",
  "cron": "0 10 1 * *",
  "session": "isolated",
  "message": "Prepare the monthly review. Summarize the past month from weekly reviews and daily logs. Write to daily log and alert me.",
  "announce": true,
  "channel": "telegram"
}
```
