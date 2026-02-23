import 'dotenv/config';
import express from 'express';
import { testConnection } from './db/client';
import { webhookRouter } from './routes/webhooks';
import { escrowRouter } from './routes/escrows';

const app  = express();
const PORT = process.env.PORT || 3000;

app.use(express.json());

// Health check
app.get('/health', (_, res) => {
  res.json({ status: 'ok', service: 'streampay-backend', ts: new Date() });
});

// Routes
app.use('/webhook', webhookRouter);
app.use('/escrows', escrowRouter);

// Start
async function start() {
  await testConnection();
  app.listen(PORT, () => {
    console.log(`🚀 StreamPay backend running on port ${PORT}`);
  });
}

start().catch(err => {
  console.error('Failed to start:', err);
  process.exit(1);
});
