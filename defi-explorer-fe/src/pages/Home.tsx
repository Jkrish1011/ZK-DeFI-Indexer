import { Layout } from "@/components/Layout";
import { StatCard } from "@/components/StatCard";
import { Button } from "@/components/ui/button";
import { Box, ArrowRightLeft, Layers, Activity } from "lucide-react";
import { Link } from "react-router-dom";

const Home = () => {
  return (
    <Layout>
      <div className="space-y-8">
        <div className="space-y-2">
          <h1 className="text-4xl font-bold">Rollup Explorer</h1>
          <p className="text-lg text-muted-foreground">
            Real-time indexer for Arbitrum batch data with Uniswap V3 DEX analytics
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <StatCard
            title="Latest Batch"
            value="12,847"
            icon={Box}
            description="Sequence number"
            trend={{ value: 2.5, label: "from last hour" }}
          />
          <StatCard
            title="L1 Block Synced"
            value="18,234,567"
            icon={Activity}
            description="Current block height"
          />
          <StatCard
            title="Indexed Transactions"
            value="2.4M"
            icon={ArrowRightLeft}
            description="Total decoded L2 txs"
            trend={{ value: 5.2, label: "today" }}
          />
          <StatCard
            title="Uniswap Events"
            value="847K"
            icon={Layers}
            description="Swaps, mints, burns"
          />
        </div>

        <div className="bg-card border border-border rounded-lg p-6 space-y-4">
          <h2 className="text-2xl font-semibold">What is this?</h2>
          <p className="text-muted-foreground">
            This indexer processes Arbitrum Nitro batch data from Ethereum L1, decompresses blob payloads,
            decodes transactions, and extracts Uniswap V3 DEX activity into a normalized database.
            It handles chain reorgs, tracks confirmations, and provides real-time metrics.
          </p>
          <div className="flex flex-wrap gap-3">
            <Link to="/batches">
              <Button variant="default">
                <Box className="h-4 w-4 mr-2" />
                Browse Batches
              </Button>
            </Link>
            <Link to="/pools">
              <Button variant="secondary">
                <Layers className="h-4 w-4 mr-2" />
                Explore Uniswap
              </Button>
            </Link>
            <Link to="/docs">
              <Button variant="outline">
                View Documentation
              </Button>
            </Link>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-card border border-border rounded-lg p-6 space-y-3">
            <h3 className="text-xl font-semibold flex items-center gap-2">
              <Box className="h-5 w-5 text-primary" />
              Batch Processing
            </h3>
            <p className="text-sm text-muted-foreground">
              Track L1 commitments, decompress 4844 blobs, parse Nitro batch formats,
              and extract individual L2 transactions with full metadata.
            </p>
            <Link to="/batches">
              <Button variant="link" className="p-0">View batches →</Button>
            </Link>
          </div>

          <div className="bg-card border border-border rounded-lg p-6 space-y-3">
            <h3 className="text-xl font-semibold flex items-center gap-2">
              <Layers className="h-5 w-5 text-accent" />
              DEX Analytics
            </h3>
            <p className="text-sm text-muted-foreground">
              Decoded Uniswap V3 swaps, liquidity events, and pool activity.
              Explore normalized event data with token addresses, amounts, and prices.
            </p>
            <Link to="/pools">
              <Button variant="link" className="p-0">Explore pools →</Button>
            </Link>
          </div>
        </div>
      </div>
    </Layout>
  );
};

export default Home;
