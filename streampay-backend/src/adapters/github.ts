import { Request, Response } from 'express';
import { TaskEvent } from '../types';
import { processTaskEvent } from '../services/matcher';

export async function handleGithubWebhook(req: Request, res: Response): Promise<void> {
  const event   = req.headers['x-github-event'] as string;
  const payload = req.body;

  console.log(`📨 GitHub event: ${event}, action: ${payload.action}`);

  if (event === 'issues' && payload.action === 'closed') {
    const issue    = payload.issue;
    const repoName = payload.repository.full_name;

    console.log(`🔍 Issue closed: "${issue.title}" in ${repoName}`);

    const taskEvent: TaskEvent = {
      platform:   'github',
      externalId: repoName,
      taskId:     String(issue.number),
      taskTitle:  issue.title,
      taskLabels: issue.labels?.map((l: any) => l.name) || [],
      taskUrl:    issue.html_url,
      isDone:     true,
      rawPayload: payload,
    };

    res.json({ received: true });
    await processTaskEvent(taskEvent);
    return;
  }

  res.json({ received: true, event, skipped: true });
}
