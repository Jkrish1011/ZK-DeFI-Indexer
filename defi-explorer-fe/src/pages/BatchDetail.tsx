import { Layout } from "@/components/Layout";
import { TechnicalText } from "@/components/TechnicalText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ArrowLeft, Filter } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { useState } from "react";

const BatchDetail = () => {
  const { id } = useParams();
  const [filterUniswap, setFilterUniswap] = useState(false);

  // Mock data
  const batch = {
    sequence: Number(id),
    l1Block: 18234567,
    l1TxHash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
    commitment: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
    status: 'confirmed',
    timestamp: new Date().toISOString(),
    txCount: 342,
  };

  const transactions = Array.from({ length: 15 }, (_, i) => ({
    hash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
    from: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
    to: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
    method: ['swapExactTokensForTokens', 'mint', 'burn', 'transfer'][Math.floor(Math.random() * 4)],
    isUniswap: Math.random() > 0.5,
  }));

  const filteredTxs = filterUniswap ? transactions.filter(tx => tx.isUniswap) : transactions;

  return (
    <Layout>
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Link to="/batches">
            <Button variant="ghost" size="sm">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Batches
            </Button>
          </Link>
        </div>

        <div>
          <h1 className="text-3xl font-bold">Batch #{batch.sequence}</h1>
          <p className="text-muted-foreground mt-1">
            Detailed view of batch metadata and transactions
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Batch Information</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <p className="text-sm text-muted-foreground">Sequence Number</p>
                <p className="font-semibold">#{batch.sequence}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">L1 Block</p>
                <p className="font-semibold">{batch.l1Block.toLocaleString()}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Status</p>
                <Badge variant="default">{batch.status}</Badge>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Transaction Count</p>
                <p className="font-semibold">{batch.txCount}</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Technical Details</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <p className="text-sm text-muted-foreground mb-1">L1 Transaction Hash</p>
                <TechnicalText text={batch.l1TxHash} />
              </div>
              <div>
                <p className="text-sm text-muted-foreground mb-1">Commitment</p>
                <TechnicalText text={batch.commitment} />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Timestamp</p>
                <p className="font-mono text-xs">{new Date(batch.timestamp).toLocaleString()}</p>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-2xl font-semibold">Transactions</h2>
            <Button
              variant={filterUniswap ? "default" : "outline"}
              size="sm"
              onClick={() => setFilterUniswap(!filterUniswap)}
            >
              <Filter className="h-4 w-4 mr-2" />
              {filterUniswap ? "Show All" : "Uniswap Only"}
            </Button>
          </div>

          <div className="bg-card border border-border rounded-lg">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Tx Hash</TableHead>
                  <TableHead>From</TableHead>
                  <TableHead>To</TableHead>
                  <TableHead>Method</TableHead>
                  <TableHead>Type</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredTxs.map((tx) => (
                  <TableRow key={tx.hash}>
                    <TableCell>
                      <Link to={`/transactions/${tx.hash}`}>
                        <TechnicalText text={tx.hash} truncate />
                      </Link>
                    </TableCell>
                    <TableCell>
                      <TechnicalText text={tx.from} truncate />
                    </TableCell>
                    <TableCell>
                      <TechnicalText text={tx.to} truncate />
                    </TableCell>
                    <TableCell>
                      <code className="text-xs">{tx.method}</code>
                    </TableCell>
                    <TableCell>
                      {tx.isUniswap && (
                        <Badge variant="secondary">Uniswap</Badge>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      </div>
    </Layout>
  );
};

export default BatchDetail;
