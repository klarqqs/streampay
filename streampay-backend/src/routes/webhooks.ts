import { Router, Request, Response } from 'express';

export const webhookRouter = Router();

webhookRouter.post('/:platform', async (req: Request, res: Response) => {
  const { platform } = req.params;
  console.log(`📨 Webhook received from: ${platform}`);

  // Adapters plugged in next phase
  res.json({ received: true, platform });
});
