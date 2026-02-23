import {
  Contract,
  Keypair,
  nativeToScVal,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  BASE_FEE,
} from '@stellar/stellar-sdk';

const RPC_URL     = process.env.STELLAR_RPC_URL!;
const NETWORK     = process.env.STELLAR_NETWORK === 'mainnet'
  ? Networks.PUBLIC
  : Networks.TESTNET;

export async function markComplete(
  contractId:     string,
  milestoneIndex: number,
  taskUrl:        string
): Promise<string> {
  const keypair  = Keypair.fromSecret(process.env.BACKEND_SECRET_KEY!);
  const server   = new SorobanRpc.Server(RPC_URL);
  const account  = await server.getAccount(keypair.publicKey());
  const contract = new Contract(contractId);

  const tx = new TransactionBuilder(account, {
    fee:        BASE_FEE,
    networkPassphrase: NETWORK,
  })
    .addOperation(contract.call(
      'mark_complete',
      nativeToScVal(milestoneIndex, { type: 'u32' }),
      nativeToScVal(taskUrl,        { type: 'string' }),
    ))
    .setTimeout(30)
    .build();

  const prepared = await server.prepareTransaction(tx);
  prepared.sign(keypair);

  const result = await server.sendTransaction(prepared);
  console.log(`✅ mark_complete submitted: ${result.hash}`);
  return result.hash;
}
