import { Layout } from "@/components/Layout";
import { StatCard } from "@/components/StatCard";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Box, ArrowRightLeft, Layers, Activity, Clock, CheckCircle2 } from "lucide-react";
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";

const Metrics = () => {
  // Mock data
  const batchesProcessedData = Array.from({ length: 24 }, (_, i) => ({
    hour: `${i}:00`,
    batches: Math.floor(Math.random() * 50) + 20,
  }));

  const eventsData = Array.from({ length: 7 }, (_, i) => ({
    day: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'][i],
    swaps: Math.floor(Math.random() * 5000) + 2000,
    mints: Math.floor(Math.random() * 1000) + 500,
    burns: Math.floor(Math.random() * 800) + 300,
  }));

  return (
    <Layout>
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">System Metrics</h1>
          <p className="text-muted-foreground mt-1">
            Backend sync status and indexer performance
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <StatCard
            title="Total Batches Processed"
            value="12,847"
            icon={Box}
            description="Since deployment"
            trend={{ value: 2.5, label: "from yesterday" }}
          />
          <StatCard
            title="L1 Block Synced"
            value="18,234,567"
            icon={Activity}
            description="Current sync height"
          />
          <StatCard
            title="Indexed Transactions"
            value="2.4M"
            icon={ArrowRightLeft}
            description="Total L2 txs decoded"
            trend={{ value: 5.2, label: "this week" }}
          />
          <StatCard
            title="Uniswap Events"
            value="847K"
            icon={Layers}
            description="Swaps + liquidity changes"
            trend={{ value: 3.8, label: "this week" }}
          />
          <StatCard
            title="Pending Confirmations"
            value="3"
            icon={Clock}
            description="Awaiting finality"
          />
          <StatCard
            title="Reorgs Handled"
            value="12"
            icon={CheckCircle2}
            description="Chain reorganizations"
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card>
            <CardHeader>
              <CardTitle>Batches Processed (Last 24h)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={batchesProcessedData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                  <XAxis dataKey="hour" stroke="hsl(var(--muted-foreground))" />
                  <YAxis stroke="hsl(var(--muted-foreground))" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                    }}
                  />
                  <Line
                    type="monotone"
                    dataKey="batches"
                    stroke="hsl(var(--primary))"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Uniswap Events (Last 7 Days)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={eventsData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                  <XAxis dataKey="day" stroke="hsl(var(--muted-foreground))" />
                  <YAxis stroke="hsl(var(--muted-foreground))" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                    }}
                  />
                  <Bar dataKey="swaps" fill="hsl(var(--primary))" />
                  <Bar dataKey="mints" fill="hsl(var(--success))" />
                  <Bar dataKey="burns" fill="hsl(var(--accent))" />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Sync Status</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between p-4 bg-muted/30 rounded-lg">
              <div>
                <p className="font-semibold">Backend Status</p>
                <p className="text-sm text-muted-foreground">All systems operational</p>
              </div>
              <CheckCircle2 className="h-6 w-6 text-success" />
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="p-4 bg-muted/30 rounded-lg">
                <p className="text-sm text-muted-foreground">Avg. Batch Time</p>
                <p className="text-2xl font-bold">2.3s</p>
              </div>
              <div className="p-4 bg-muted/30 rounded-lg">
                <p className="text-sm text-muted-foreground">Avg. Decode Time</p>
                <p className="text-2xl font-bold">0.8s</p>
              </div>
              <div className="p-4 bg-muted/30 rounded-lg">
                <p className="text-sm text-muted-foreground">Events/Second</p>
                <p className="text-2xl font-bold">124</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </Layout>
  );
};

export default Metrics;
