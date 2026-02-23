import { Router, Request, Response } from 'express';
import { handleGithubWebhook } from '../adapters/github';

export const webhookRouter = Router();

webhookRouter.post('/github', async (req: Request, res: Response) => {
  const event = req.headers['x-github-event'] as string;
  console.log(`📨 GitHub event: ${event}`);
  console.log(`📦 Payload: ${JSON.stringify(req.body, null, 2)}`);

  if (event === 'projects_v2_item') {
    await handleGithubWebhook(req, res);
  } else {
    res.json({ received: true, event });
  }
});
