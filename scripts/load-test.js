// k6 load test script for OpenClaw Router
// Run with: k6 run scripts/load-test.js
//
// Requires k6 installed: https://k6.io/docs/get-started/installation/
//
// Environment variables:
//   BASE_URL  - Gateway URL (default: http://localhost:8080)
//   API_KEY   - API key for authentication (default: test-key)

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const routingLatency = new Trend('routing_latency');

export const options = {
  stages: [
    { duration: '30s', target: 50 },   // Ramp up to 50 VUs
    { duration: '1m', target: 100 },    // Hold at 100 VUs
    { duration: '30s', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<500'],   // 99th percentile < 500ms
    http_req_failed: ['rate<0.01'],     // Error rate < 1%
    errors: ['rate<0.05'],              // Custom error rate < 5%
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-key';

const headers = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${API_KEY}`,
};

// Test scenarios with varying complexity
const scenarios = [
  // Simple query
  {
    model: 'gpt-4o-mini',
    messages: [{ role: 'user', content: 'Hello, how are you?' }],
  },
  // Medium complexity
  {
    model: 'gpt-4o',
    messages: [
      { role: 'system', content: 'You are a helpful coding assistant.' },
      { role: 'user', content: 'Write a function to sort an array in JavaScript.' },
    ],
    max_tokens: 500,
  },
  // Complex with tools
  {
    messages: [
      { role: 'user', content: 'Analyze this data and provide insights.' },
    ],
    tools: [
      {
        type: 'function',
        function: {
          name: 'analyze_data',
          description: 'Analyze a dataset',
          parameters: {
            type: 'object',
            properties: {
              data: { type: 'string' },
            },
          },
        },
      },
    ],
  },
  // Eco cost profile
  {
    messages: [{ role: 'user', content: 'What is 2+2?' }],
    x_cost_profile: 'eco',
  },
  // Premium cost profile
  {
    messages: [
      {
        role: 'user',
        content: 'Analyze the pros and cons of microservices architecture. Think step by step.',
      },
    ],
    x_cost_profile: 'premium',
    x_tier_hint: 'enterprise',
  },
];

export default function () {
  // Pick a random scenario
  const scenario = scenarios[Math.floor(Math.random() * scenarios.length)];
  const payload = JSON.stringify(scenario);

  const startTime = Date.now();
  const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, {
    headers: headers,
  });
  const duration = Date.now() - startTime;

  routingLatency.add(duration);

  const success = check(res, {
    'status is 200 or 502': (r) => r.status === 200 || r.status === 502,
    'response has body': (r) => r.body && r.body.length > 0,
    'response is JSON': (r) => {
      try {
        JSON.parse(r.body);
        return true;
      } catch (e) {
        return false;
      }
    },
  });

  errorRate.add(!success);

  // Check router metadata if present
  if (res.status === 200) {
    try {
      const body = JSON.parse(res.body);
      check(body, {
        'has model field': (b) => b.model !== undefined,
        'has choices': (b) => b.choices && b.choices.length > 0,
        'has usage': (b) => b.usage !== undefined,
      });
    } catch (e) {
      // Response parsing failed
    }
  }

  sleep(0.1);
}

// Health check test
export function healthCheck() {
  const res = http.get(`${BASE_URL}/health`);
  check(res, {
    'health check status is 200': (r) => r.status === 200,
  });
}
