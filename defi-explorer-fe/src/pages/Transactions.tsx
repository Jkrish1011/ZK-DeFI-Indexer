import { Layout } from "@/components/Layout";
import { TechnicalText } from "@/components/TechnicalText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Link, useSearchParams } from "react-router-dom";

// Mock data
const mockTransactions = Array.from({ length: 20 }, (_, i) => ({
  hash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
  from: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
  to: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
  method: ['swapExactTokensForTokens', 'mint', 'burn', 'transfer', 'approve'][Math.floor(Math.random() * 5)],
  value: Math.random() > 0.7 ? `${(Math.random() * 10).toFixed(4)} ETH` : '-',
  batch: Math.floor(Math.random() * 1000) + 12000,
}));

const Transactions = () => {
  const [searchParams] = useSearchParams();
  const searchQuery = searchParams.get('search');

  return (
    <Layout>
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Transactions</h1>
          <p className="text-muted-foreground mt-1">
            {searchQuery
              ? `Search results for: "${searchQuery}"`
              : "Browse recent decoded L2 transactions"}
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Tx Hash</TableHead>
                <TableHead>Batch</TableHead>
                <TableHead>From</TableHead>
                <TableHead>To</TableHead>
                <TableHead>Method</TableHead>
                <TableHead>Value</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mockTransactions.map((tx) => (
                <TableRow key={tx.hash} className="cursor-pointer hover:bg-muted/50">
                  <TableCell>
                    <Link to={`/transactions/${tx.hash}`} className="text-primary hover:underline">
                      <TechnicalText text={tx.hash} truncate />
                    </Link>
                  </TableCell>
                  <TableCell>
                    <Link to={`/batches/${tx.batch}`} className="text-primary hover:underline">
                      #{tx.batch}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <TechnicalText text={tx.from} truncate />
                  </TableCell>
                  <TableCell>
                    <TechnicalText text={tx.to} truncate />
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="font-mono text-xs">
                      {tx.method}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-semibold">{tx.value}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            Showing 1-20 of 2,431,847 transactions
          </p>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" disabled>
              <ChevronLeft className="h-4 w-4" />
              Previous
            </Button>
            <Button variant="outline" size="sm">
              Next
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>
    </Layout>
  );
};

export default Transactions;
