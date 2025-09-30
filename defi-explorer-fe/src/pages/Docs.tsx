import { Layout } from "@/components/Layout";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Code2, Database, Layers, Zap } from "lucide-react";

const Docs = () => {
  return (
    <Layout>
      <div className="max-w-4xl space-y-8">
        <div>
          <h1 className="text-4xl font-bold">Documentation</h1>
          <p className="text-lg text-muted-foreground mt-2">
            Learn how to use the Arbitrum Rollup Explorer and understand the indexer architecture
          </p>
        </div>

        <Card>
          <CardContent className="prose prose-invert max-w-none pt-6">
            <h2 className="flex items-center gap-2 text-2xl font-bold text-foreground">
              <Code2 className="h-6 w-6 text-primary" />
              What is this project?
            </h2>
            <p className="text-muted-foreground">
              This is a comprehensive blockchain indexer for Arbitrum Nitro batches. It processes L1 batch commitments,
              decompresses EIP-4844 blob data, parses Nitro batch formats, decodes L2 transactions, and extracts
              Uniswap V3 DEX activity into a normalized PostgreSQL database.
            </p>

            <Separator className="my-6" />

            <h2 className="flex items-center gap-2 text-2xl font-bold text-foreground">
              <Layers className="h-6 w-6 text-primary" />
              How it works
            </h2>
            <div className="space-y-4">
              <div className="bg-muted/30 p-4 rounded-lg">
                <h3 className="text-lg font-semibold text-foreground mb-2">1. L1 Monitoring</h3>
                <p className="text-sm text-muted-foreground">
                  Monitors Ethereum L1 for Arbitrum batch commitments via SequencerInbox contract events.
                  Tracks confirmations and handles chain reorganizations.
                </p>
              </div>

              <div className="bg-muted/30 p-4 rounded-lg">
                <h3 className="text-lg font-semibold text-foreground mb-2">2. Blob Decompression</h3>
                <p className="text-sm text-muted-foreground">
                  Fetches EIP-4844 blob data from Beacon Chain, decompresses using brotli, and extracts
                  the raw batch payload.
                </p>
              </div>

              <div className="bg-muted/30 p-4 rounded-lg">
                <h3 className="text-lg font-semibold text-foreground mb-2">3. Batch Parsing</h3>
                <p className="text-sm text-muted-foreground">
                  Parses Nitro batch format headers, extracts individual L2 transactions with block context,
                  and decodes RLP-encoded transaction data (both legacy and typed transactions).
                </p>
              </div>

              <div className="bg-muted/30 p-4 rounded-lg">
                <h3 className="text-lg font-semibold text-foreground mb-2">4. ABI Decoding</h3>
                <p className="text-sm text-muted-foreground">
                  Identifies Uniswap V3 contract calls, decodes function parameters and event logs,
                  normalizes swap/mint/burn data with token addresses, amounts, and prices.
                </p>
              </div>
            </div>

            <Separator className="my-6" />

            <h2 className="flex items-center gap-2 text-2xl font-bold text-foreground">
              <Database className="h-6 w-6 text-primary" />
              API Endpoints
            </h2>
            <p className="text-muted-foreground">
              The backend exposes the following REST API endpoints:
            </p>

            <div className="space-y-3 not-prose">
              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/batches</code>
                <p className="text-sm text-muted-foreground mt-2">
                  List batches with pagination, filtering by status or block range
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/batches/:sequence</code>
                <p className="text-sm text-muted-foreground mt-2">
                  Get detailed batch metadata and list of transactions
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/transactions</code>
                <p className="text-sm text-muted-foreground mt-2">
                  Search transactions by hash, address, or method signature
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/transactions/:hash</code>
                <p className="text-sm text-muted-foreground mt-2">
                  Get full transaction details with decoded call data
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/pools</code>
                <p className="text-sm text-muted-foreground mt-2">
                  List Uniswap V3 pools with activity metrics
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/pools/:address</code>
                <p className="text-sm text-muted-foreground mt-2">
                  Get pool details, recent swaps, and liquidity events
                </p>
              </div>

              <div className="bg-card border border-border p-4 rounded-lg">
                <code className="text-primary font-mono text-sm">GET /api/metrics</code>
                <p className="text-sm text-muted-foreground mt-2">
                  System health, sync status, and processing metrics
                </p>
              </div>
            </div>

            <Separator className="my-6" />

            <h2 className="flex items-center gap-2 text-2xl font-bold text-foreground">
              <Zap className="h-6 w-6 text-primary" />
              Using the Explorer
            </h2>
            
            <h3 className="text-xl font-semibold text-foreground">Navigation</h3>
            <ul className="text-muted-foreground space-y-2">
              <li><strong className="text-foreground">Home</strong> - Overview with quick stats and CTA links</li>
              <li><strong className="text-foreground">Batches</strong> - Browse all batches, click for detailed view</li>
              <li><strong className="text-foreground">Transactions</strong> - Search by hash/address, view decoded data</li>
              <li><strong className="text-foreground">Pools</strong> - Explore Uniswap activity by pool</li>
              <li><strong className="text-foreground">Metrics</strong> - System health and performance charts</li>
            </ul>

            <h3 className="text-xl font-semibold text-foreground mt-6">Search</h3>
            <p className="text-muted-foreground">
              Use the search bar in the top navigation to quickly find:
            </p>
            <ul className="text-muted-foreground space-y-1">
              <li>• Transaction by hash (0x...)</li>
              <li>• Batch by sequence number (#12847)</li>
              <li>• Activity by address (0x...)</li>
            </ul>

            <Separator className="my-6" />

            <h2 className="text-2xl font-bold text-foreground">Future Roadmap</h2>
            <div className="space-y-2">
              <div className="flex items-start gap-2">
                <div className="h-6 w-6 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                  <div className="h-2 w-2 rounded-full bg-primary" />
                </div>
                <div>
                  <p className="font-semibold text-foreground">KZG Opening Verification</p>
                  <p className="text-sm text-muted-foreground">Verify blob commitments against KZG proofs</p>
                </div>
              </div>

              <div className="flex items-start gap-2">
                <div className="h-6 w-6 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                  <div className="h-2 w-2 rounded-full bg-primary" />
                </div>
                <div>
                  <p className="font-semibold text-foreground">ZK-based Index Proofs</p>
                  <p className="text-sm text-muted-foreground">Generate zero-knowledge proofs of indexed state</p>
                </div>
              </div>

              <div className="flex items-start gap-2">
                <div className="h-6 w-6 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                  <div className="h-2 w-2 rounded-full bg-primary" />
                </div>
                <div>
                  <p className="font-semibold text-foreground">Multi-DEX Support</p>
                  <p className="text-sm text-muted-foreground">Expand to SushiSwap, Curve, and other protocols</p>
                </div>
              </div>

              <div className="flex items-start gap-2">
                <div className="h-6 w-6 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                  <div className="h-2 w-2 rounded-full bg-primary" />
                </div>
                <div>
                  <p className="font-semibold text-foreground">Historical Analytics</p>
                  <p className="text-sm text-muted-foreground">Long-term metrics and trend analysis</p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </Layout>
  );
};

export default Docs;
