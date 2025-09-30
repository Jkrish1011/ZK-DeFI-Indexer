import { Home, Box, ArrowRightLeft, Layers, Activity, BookOpen } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  Sidebar as SidebarUI,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";

const navItems = [
  { title: "Home", url: "/", icon: Home },
  { title: "Batches", url: "/batches", icon: Box },
  { title: "Transactions", url: "/transactions", icon: ArrowRightLeft },
  { title: "Pools", url: "/pools", icon: Layers },
  { title: "Metrics", url: "/metrics", icon: Activity },
  { title: "Docs", url: "/docs", icon: BookOpen },
];

export function Sidebar() {
  const { open } = useSidebar();

  return (
    <SidebarUI className="border-r border-sidebar-border">
      <SidebarContent>
        <div className="p-4 border-b border-sidebar-border">
          {open ? (
            <div>
              <h1 className="text-lg font-bold text-primary">Rollup Explorer</h1>
              <p className="text-xs text-muted-foreground">L2 Batch Indexer</p>
            </div>
          ) : (
            <div className="text-primary font-bold text-xl">RE</div>
          )}
        </div>

        <SidebarGroup>
          <SidebarGroupLabel>Navigation</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton asChild>
                    <NavLink
                      to={item.url}
                      end={item.url === "/"}
                      className={({ isActive }) =>
                        isActive
                          ? "bg-sidebar-accent text-sidebar-accent-foreground"
                          : "hover:bg-sidebar-accent/50"
                      }
                    >
                      <item.icon className="h-4 w-4" />
                      {open && <span>{item.title}</span>}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </SidebarUI>
  );
}
