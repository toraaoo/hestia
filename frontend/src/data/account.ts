export interface Account {
  name: string;
  kind: string;
}

const MOCK_ACCOUNT: Account = { name: "Steve_Builds", kind: "Microsoft account" };

export const useAccount = (): Account => MOCK_ACCOUNT;
