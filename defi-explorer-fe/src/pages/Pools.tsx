import { Layout } from "@/components/Layout";
import { TechnicalText } from "@/components/TechnicalText";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ChevronLeft, ChevronRight, TrendingUp } from "lucide-react";
import { Link } from "react-router-dom";

// Mock data
const mockPools = Array.from({ length: 20 }, (_, i) => ({
  address: `0x${Math.random().toString(16).slice(2).padEnd(40, '0')}`,
  token0: ['WETH', 'USDC', 'DAI', 'WBTC'][Math.floor(Math.random() * 4)],
  token1: ['USDC', 'USDT', 'DAI', 'WETH'][Math.floor(Math.random() * 4)],
  fee: ['0.05%', '0.30%', '1.00%'][Math.floor(Math.random() * 3)],
  swaps: Math.floor(Math.random() * 10000) + 1000,
  liquidity: Math.floor(Math.random() * 50000) + 10000,
  volume24h: `$${(Math.random() * 1000000).toFixed(0)}`,
}));

const Pools = () => {
  return (
    <Layout>
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Uniswap V3 Pools</h1>
          <p className="text-muted-foreground mt-1">
            Explore DEX activity with decoded swap and liquidity events
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Pool Address</TableHead>
                <TableHead>Pair</TableHead>
                <TableHead>Fee</TableHead>
                <TableHead>Swaps</TableHead>
                <TableHead>Liquidity Events</TableHead>
                <TableHead>24h Volume</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mockPools.map((pool) => (
                <TableRow key={pool.address} className="cursor-pointer hover:bg-muted/50">
                  <TableCell>
                    <Link to={`/pools/${pool.address}`} className="text-primary hover:underline">
                      <TechnicalText text={pool.address} truncate />
                    </Link>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <span className="font-semibold">{pool.token0}/{pool.token1}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-2 py-1 rounded">{pool.fee}</code>
                  </TableCell>
                  <TableCell>{pool.swaps.toLocaleString()}</TableCell>
                  <TableCell>{pool.liquidity.toLocaleString()}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <TrendingUp className="h-3 w-3 text-success" />
                      <span className="font-semibold">{pool.volume24h}</span>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            Showing 1-20 of 847 active pools
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

export default Pools;
