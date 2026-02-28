/**
 * OpenClaw Router - Integration Tests (Real User Simulation)
 */

import OpenClawRouter from './index';

/**
 * Simulate real user workflow
 */
async function testRealUserWorkflow() {
  console.log('🧪 Testing Unified Router as Real User\n');
  console.log('=' .repeat(70));
  
  try {
    // Test 1: Wallet Creation
    console.log('\n📋 Test 1: Wallet Creation');
    const walletResp = await OpenClawRouter.handleTelegramMessage({
      text: '/wallet create',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 1
    });
    console.log('✅ Response:', walletResp.substring(0, 100));
    
    // Test 2: Top-up Credits
    console.log('\n💳 Test 2: Top-up Credits');
    const topupResp = await OpenClawRouter.handleTelegramMessage({
      text: '/topup 100',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 2
    });
    console.log('✅ Response:', topupResp.substring(0, 100));
    
    // Test 3: Check Stats
    console.log('\n📊 Test 3: Check Statistics');
    const statsResp = await OpenClawRouter.handleTelegramMessage({
      text: '/stats',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 3
    });
    console.log('✅ Response:', statsResp.substring(0, 100));
    
    // Test 4: LLM Request
    console.log('\n🤖 Test 4: LLM Request');
    const llmResp = await OpenClawRouter.handleTelegramMessage({
      text: 'What is the capital of France?',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 4
    });
    console.log('✅ Response:', llmResp.substring(0, 150));
    
    // Test 5: Model Selection
    console.log('\n🎯 Test 5: Model Selection');
    const modelResp = await OpenClawRouter.handleTelegramMessage({
      text: '/model list',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 5
    });
    console.log('✅ Response:', modelResp.substring(0, 100));
    
    // Test 6: Multi-turn Conversation
    console.log('\n💬 Test 6: Multi-turn Conversation');
    const follow1 = await OpenClawRouter.handleTelegramMessage({
      text: 'Tell me more about French history',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 6
    });
    console.log('✅ Follow-up 1:', follow1.substring(0, 100));
    
    const follow2 = await OpenClawRouter.handleTelegramMessage({
      text: 'What year was the French Revolution?',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 7
    });
    console.log('✅ Follow-up 2:', follow2.substring(0, 100));
    
    // Test 7: Discord Integration
    console.log('\n🎮 Test 7: Discord Integration');
    const discordResp = await OpenClawRouter.handleDiscordMessage({
      content: 'Hello from Discord!',
      author: { id: 'discord_user_123', username: 'discorduser' },
      id: 'msg_discord_1'
    });
    console.log('✅ Discord Response:', discordResp.substring(0, 100));
    
    // Test 8: Help Command
    console.log('\n❓ Test 8: Help Command');
    const helpResp = await OpenClawRouter.handleTelegramMessage({
      text: '/help',
      from: { id: 7895653822, username: 'narasamma' },
      message_id: 8
    });
    console.log('✅ Help:', helpResp.substring(0, 150));
    
    // Test 9: Gateway Health Check
    console.log('\n💚 Test 9: Gateway Health');
    const health = await OpenClawRouter.getHealth();
    console.log('✅ Gateway Status:', health);
    
    // Test 10: Metrics
    console.log('\n📈 Test 10: Performance Metrics');
    const metrics = OpenClawRouter.getMetrics();
    console.log('✅ Metrics:', {
      totalRequests: metrics.totalRequests,
      totalCostUSD: metrics.totalCostUSD.toFixed(4),
      cacheHitRate: (metrics.cacheHitRate * 100).toFixed(1) + '%'
    });
    
    console.log('\n' + '='.repeat(70));
    console.log('✅ ALL TESTS PASSED\n');
    
  } catch (error: any) {
    console.error('❌ Test Failed:', error.message);
    process.exit(1);
  }
}

// Run tests
testRealUserWorkflow().catch(console.error);
