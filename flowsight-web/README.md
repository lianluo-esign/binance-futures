# FlowSight Web

A modern, high-performance web application for professional trading analysis, built with Next.js 14 and TypeScript. This application provides real-time order book visualization, active order charts, and footprint charts for comprehensive market analysis.

## Features

- **Real-time Trading Interface**: Three-panel layout optimized for professional trading analysis
- **Order Book Visualization**: Live bid/ask volumes with color-coded bars
- **Active Order Chart**: Real-time line chart with volume-proportional dots
- **Footprint Chart**: 5-minute candlestick charts with price level volume accumulation
- **Technical Analysis**: Real-time indicators including volatility, jump signals, and momentum
- **Responsive Design**: Optimized for desktop, tablet, and mobile devices
- **High Performance**: Sub-100ms latency with efficient memory management

## Technology Stack

- **Frontend**: Next.js 14, React 19, TypeScript 5
- **Styling**: Tailwind CSS 4 with custom trading theme
- **State Management**: Zustand for lightweight, performant state management
- **Visualizations**: D3.js for complex data visualizations
- **WebSocket**: Native WebSocket with automatic reconnection
- **Testing**: Jest with React Testing Library
- **Code Quality**: ESLint, Prettier, TypeScript strict mode

## Getting Started

### Prerequisites

- Node.js 18+ 
- npm or yarn

### Installation

1. Install dependencies:
```bash
npm install
```

2. Run the development server:
```bash
npm run dev
```

3. Open [http://localhost:3000](http://localhost:3000) in your browser

### Available Scripts

- `npm run dev` - Start development server with Turbopack
- `npm run build` - Build for production
- `npm run start` - Start production server
- `npm run lint` - Run ESLint
- `npm run lint:fix` - Fix ESLint issues
- `npm run type-check` - Run TypeScript type checking
- `npm run test` - Run Jest tests
- `npm run test:watch` - Run tests in watch mode
- `npm run test:coverage` - Run tests with coverage report

## Project Structure

```
src/
├── app/                 # Next.js App Router pages
├── components/          # React components
│   ├── charts/         # Chart components (D3.js visualizations)
│   ├── layout/         # Layout components
│   ├── trading/        # Trading-specific components
│   └── ui/             # Reusable UI components
├── hooks/              # Custom React hooks
├── lib/                # Utility libraries and constants
├── services/           # Business logic and external services
├── stores/             # Zustand state management
├── types/              # TypeScript type definitions
└── utils/              # Helper functions
```

## Configuration

### Environment Variables

Create a `.env.local` file for local development:

```env
# Add environment variables as needed
NEXT_PUBLIC_WS_URL=wss://stream.binance.com:9443/ws
```

### Tailwind CSS

The application uses a custom Tailwind configuration optimized for trading interfaces with:
- Custom color scheme for buy/sell indicators
- Trading-specific breakpoints
- Panel proportion utilities
- Animation classes for real-time updates

## Development Guidelines

### Code Style

- Use TypeScript strict mode
- Follow ESLint and Prettier configurations
- Use meaningful component and variable names
- Write comprehensive JSDoc comments for complex functions

### Performance

- Maintain 60fps rendering performance
- Keep memory usage under 200MB
- Process WebSocket messages within 50ms
- Implement proper cleanup for subscriptions and timers

### Testing

- Write unit tests for business logic
- Use React Testing Library for component tests
- Mock WebSocket connections in tests
- Maintain test coverage above 80%

## Deployment

### Production Build

```bash
npm run build
npm run start
```

### Docker Deployment

```dockerfile
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build

FROM node:18-alpine AS runner
WORKDIR /app
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
EXPOSE 3000
CMD ["node", "server.js"]
```

## Contributing

1. Follow the established code style and conventions
2. Write tests for new features
3. Update documentation as needed
4. Ensure all checks pass before submitting PRs

## License

This project is private and proprietary.
