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
import { Link } from "react-router-dom";

// Mock data - replace with actual API calls
const mockBatches = Array.from({ length: 20 }, (_, i) => ({
  sequence: 12847 - i,
  l1Block: 18234567 - i * 10,
  l1TxHash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
  commitment: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
  status: Math.random() > 0.1 ? 'confirmed' : 'pending',
  txCount: Math.floor(Math.random() * 500) + 50,
}));

const Batches = () => {
  return (
    <Layout>
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Batches</h1>
          <p className="text-muted-foreground mt-1">
            Browse Arbitrum batch commitments from L1
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Sequence</TableHead>
                <TableHead>L1 Block</TableHead>
                <TableHead>L1 Tx Hash</TableHead>
                <TableHead>Commitment</TableHead>
                <TableHead>Txs</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mockBatches.map((batch) => (
                <TableRow key={batch.sequence} className="cursor-pointer hover:bg-muted/50">
                  <TableCell className="font-medium">
                    <Link to={`/batches/${batch.sequence}`} className="text-primary hover:underline">
                      #{batch.sequence}
                    </Link>
                  </TableCell>
                  <TableCell>{batch.l1Block.toLocaleString()}</TableCell>
                  <TableCell>
                    <TechnicalText text={batch.l1TxHash} truncate />
                  </TableCell>
                  <TableCell>
                    <TechnicalText text={batch.commitment} truncate />
                  </TableCell>
                  <TableCell>{batch.txCount}</TableCell>
                  <TableCell>
                    <Badge variant={batch.status === 'confirmed' ? 'default' : 'secondary'}>
                      {batch.status}
                    </Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            Showing 1-20 of 12,847 batches
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

export default Batches;
