import { Layout } from "@/components/Layout";
import { TechnicalText } from "@/components/TechnicalText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { ArrowLeft } from "lucide-react";
import { Link, useParams } from "react-router-dom";

const TransactionDetail = () => {
  const { hash } = useParams();

  // Mock data
  const transaction = {
    hash: hash || '',
    batch: 12847,
    from: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
    to: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
    value: '0.5 ETH',
    method: 'swapExactTokensForTokens',
    gasUsed: '142,384',
    status: 'success',
    timestamp: new Date().toISOString(),
  };

  const decodedParams = {
    amountIn: '1000000000000000000',
    amountOutMin: '2500000000',
    path: [
      '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
    ],
    to: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
    deadline: '1640000000',
  };

  return (
    <Layout>
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Link to="/transactions">
            <Button variant="ghost" size="sm">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Transactions
            </Button>
          </Link>
        </div>

        <div>
          <h1 className="text-3xl font-bold">Transaction Details</h1>
          <div className="mt-2">
            <TechnicalText text={transaction.hash} copyable />
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Overview</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <p className="text-sm text-muted-foreground">Status</p>
                <Badge variant="default" className="mt-1">
                  {transaction.status}
                </Badge>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Batch</p>
                <Link to={`/batches/${transaction.batch}`} className="text-primary hover:underline">
                  #{transaction.batch}
                </Link>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Timestamp</p>
                <p className="font-mono text-xs">{new Date(transaction.timestamp).toLocaleString()}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Gas Used</p>
                <p className="font-semibold">{transaction.gasUsed}</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Parties</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <p className="text-sm text-muted-foreground mb-1">From</p>
                <TechnicalText text={transaction.from} />
              </div>
              <div>
                <p className="text-sm text-muted-foreground mb-1">To</p>
                <TechnicalText text={transaction.to} />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Value</p>
                <p className="font-semibold text-lg">{transaction.value}</p>
              </div>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Decoded Call Data</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Method</p>
              <Badge variant="secondary" className="font-mono">
                {transaction.method}
              </Badge>
            </div>

            <Separator />

            <div className="space-y-3">
              <p className="font-semibold">Parameters</p>
              
              <div className="space-y-2">
                <div className="bg-muted/30 p-3 rounded">
                  <p className="text-xs text-muted-foreground">amountIn</p>
                  <code className="text-sm font-mono">{decodedParams.amountIn}</code>
                  <p className="text-xs text-muted-foreground mt-1">1.0 WETH</p>
                </div>

                <div className="bg-muted/30 p-3 rounded">
                  <p className="text-xs text-muted-foreground">amountOutMin</p>
                  <code className="text-sm font-mono">{decodedParams.amountOutMin}</code>
                  <p className="text-xs text-muted-foreground mt-1">2,500 USDC</p>
                </div>

                <div className="bg-muted/30 p-3 rounded">
                  <p className="text-xs text-muted-foreground mb-2">path</p>
                  {decodedParams.path.map((addr, i) => (
                    <div key={i} className="mb-1">
                      <TechnicalText text={addr} truncate />
                      <p className="text-xs text-muted-foreground ml-2">
                        {i === 0 ? 'WETH' : 'USDC'}
                      </p>
                    </div>
                  ))}
                </div>

                <div className="bg-muted/30 p-3 rounded">
                  <p className="text-xs text-muted-foreground">to</p>
                  <TechnicalText text={decodedParams.to} truncate />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </Layout>
  );
};

export default TransactionDetail;
