import { Toaster } from "@/components/ui/toaster";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import Home from "./pages/Home";
import Batches from "./pages/Batches";
import BatchDetail from "./pages/BatchDetail";
import Transactions from "./pages/Transactions";
import TransactionDetail from "./pages/TransactionDetail";
import Pools from "./pages/Pools";
import PoolDetail from "./pages/PoolDetail";
import Metrics from "./pages/Metrics";
import Docs from "./pages/Docs";
import NotFound from "./pages/NotFound";

const queryClient = new QueryClient();

const App = () => (
  <QueryClientProvider client={queryClient}>
    <TooltipProvider>
      <Toaster />
      <Sonner />
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/batches" element={<Batches />} />
          <Route path="/batches/:id" element={<BatchDetail />} />
          <Route path="/transactions" element={<Transactions />} />
          <Route path="/transactions/:hash" element={<TransactionDetail />} />
          <Route path="/pools" element={<Pools />} />
          <Route path="/pools/:address" element={<PoolDetail />} />
          <Route path="/metrics" element={<Metrics />} />
          <Route path="/docs" element={<Docs />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </BrowserRouter>
    </TooltipProvider>
  </QueryClientProvider>
);

export default App;
