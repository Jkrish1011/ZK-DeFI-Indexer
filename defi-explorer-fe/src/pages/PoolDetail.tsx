import { Layout } from "@/components/Layout";
import { TechnicalText } from "@/components/TechnicalText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ArrowLeft, TrendingUp, TrendingDown } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";

const PoolDetail = () => {
  const { address } = useParams();

  // Mock data
  const pool = {
    address: address || '',
    token0: 'WETH',
    token1: 'USDC',
    fee: '0.30%',
    swaps: 8547,
    liquidity: 32184,
    tvl: '$12.4M',
  };

  const swaps = Array.from({ length: 10 }, (_, i) => ({
    txHash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
    type: Math.random() > 0.5 ? 'buy' : 'sell',
    amount0: (Math.random() * 10).toFixed(4),
    amount1: (Math.random() * 25000).toFixed(2),
    price: (Math.random() * 2500 + 2000).toFixed(2),
    timestamp: new Date(Date.now() - Math.random() * 86400000).toISOString(),
  }));

  const liquidityEvents = Array.from({ length: 8 }, (_, i) => ({
    txHash: `0x${Math.random().toString(16).slice(2).padEnd(64, '0')}`,
    type: Math.random() > 0.5 ? 'mint' : 'burn',
    amount0: (Math.random() * 100).toFixed(4),
    amount1: (Math.random() * 250000).toFixed(2),
    timestamp: new Date(Date.now() - Math.random() * 86400000).toISOString(),
  }));

  const priceData = Array.from({ length: 24 }, (_, i) => ({
    time: `${i}:00`,
    price: 2500 + Math.random() * 100 - 50,
  }));

  return (
    <Layout>
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Link to="/pools">
            <Button variant="ghost" size="sm">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Pools
            </Button>
          </Link>
        </div>

        <div>
          <h1 className="text-3xl font-bold">{pool.token0}/{pool.token1} Pool</h1>
          <div className="mt-2">
            <TechnicalText text={pool.address} copyable />
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Fee Tier</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-2xl font-bold">{pool.fee}</p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Total Swaps</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-2xl font-bold">{pool.swaps.toLocaleString()}</p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Liquidity Events</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-2xl font-bold">{pool.liquidity.toLocaleString()}</p>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">TVL</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-2xl font-bold">{pool.tvl}</p>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Price Chart (Last 24h)</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={priceData}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="time" stroke="hsl(var(--muted-foreground))" />
                <YAxis stroke="hsl(var(--muted-foreground))" />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'hsl(var(--card))',
                    border: '1px solid hsl(var(--border))',
                  }}
                />
                <Line
                  type="monotone"
                  dataKey="price"
                  stroke="hsl(var(--primary))"
                  strokeWidth={2}
                  dot={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Tabs defaultValue="swaps" className="space-y-4">
          <TabsList>
            <TabsTrigger value="swaps">Recent Swaps</TabsTrigger>
            <TabsTrigger value="liquidity">Liquidity Events</TabsTrigger>
          </TabsList>

          <TabsContent value="swaps">
            <div className="bg-card border border-border rounded-lg">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Transaction</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Amount {pool.token0}</TableHead>
                    <TableHead>Amount {pool.token1}</TableHead>
                    <TableHead>Price</TableHead>
                    <TableHead>Time</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {swaps.map((swap) => (
                    <TableRow key={swap.txHash}>
                      <TableCell>
                        <Link to={`/transactions/${swap.txHash}`}>
                          <TechnicalText text={swap.txHash} truncate />
                        </Link>
                      </TableCell>
                      <TableCell>
                        <Badge variant={swap.type === 'buy' ? 'default' : 'secondary'}>
                          {swap.type === 'buy' ? (
                            <TrendingUp className="h-3 w-3 mr-1" />
                          ) : (
                            <TrendingDown className="h-3 w-3 mr-1" />
                          )}
                          {swap.type}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono">{swap.amount0}</TableCell>
                      <TableCell className="font-mono">{swap.amount1}</TableCell>
                      <TableCell className="font-semibold">${swap.price}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {new Date(swap.timestamp).toLocaleTimeString()}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </TabsContent>

          <TabsContent value="liquidity">
            <div className="bg-card border border-border rounded-lg">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Transaction</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Amount {pool.token0}</TableHead>
                    <TableHead>Amount {pool.token1}</TableHead>
                    <TableHead>Time</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {liquidityEvents.map((event) => (
                    <TableRow key={event.txHash}>
                      <TableCell>
                        <Link to={`/transactions/${event.txHash}`}>
                          <TechnicalText text={event.txHash} truncate />
                        </Link>
                      </TableCell>
                      <TableCell>
                        <Badge variant={event.type === 'mint' ? 'default' : 'secondary'}>
                          {event.type}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono">{event.amount0}</TableCell>
                      <TableCell className="font-mono">{event.amount1}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {new Date(event.timestamp).toLocaleTimeString()}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </Layout>
  );
};

export default PoolDetail;
