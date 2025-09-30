# ZK Rollup Explorer (Frontend)

This is the React/Vite frontend explorer for L2 batches submitted to Ethereum. It provides a modern UI scaffolded with Tailwind CSS and shadcn-ui components.

Folder: `defi-explorer-fe/`

## Tech Stack

- Vite
- React + TypeScript
- Tailwind CSS
- shadcn-ui

## Prerequisites

- Node.js 18+ and npm

## Project Setup

## Frontend Setup (`defi-explorer-fe/`)

1. Install dependencies:
   ```bash
   cd defi-explorer-fe
   npm install
   ```

2. Start the dev server:
   ```bash
   npm run dev
   ```

3. Other scripts:
   - Build: `npm run build`
   - Preview: `npm run preview`
   - Lint: `npm run lint`

The frontend can optionally consume data from a separate backend (not covered here). If you plan to configure environment variables for API access, create a `.env` file in this folder using Vite’s `VITE_` prefix, for example:

```env
VITE_API_BASE_URL=https://your-api.example.com
```

Then in code, you can reference it via:

```ts
const apiBase = import.meta.env.VITE_API_BASE_URL
```

## License

MIT
