# Twilio Functions Setup — Step by Step

**Goal:** Two-way voice conversation with Airy via phone.

---

## Step 1: Open Twilio Console
Go to: https://console.twilio.com
Navigate to: **Explore Products → Functions and Assets → Services**

## Step 2: Create a New Service
- Click **Create Service**
- Name it: `airy-voice`
- Click **Next**

## Step 3: Add Your Anthropic API Key
- Click **Settings** (gear icon, bottom left)
- Go to **Environment Variables**
- Click **Add**
  - Key: `ANTHROPIC_API_KEY`
  - Value: *(paste your Anthropic API key)*

## Step 4: Add the Anthropic Dependency
- Still in **Settings**, go to **Dependencies**
- Click **Add**
  - Module: `@anthropic-ai/sdk`
  - Version: `0.39.0`

## Step 5: Create the `/voice` Function
- Click **Add** → **Add Function**
- Set path to: `/voice`
- Set visibility to: **Public**
- Paste this code:

```javascript
exports.handler = function(context, event, callback) {
  const twiml = new Twilio.twiml.VoiceResponse();
  
  twiml.say({
    voice: 'Polly.Ruth',
    language: 'en-US'
  }, "Hey Emil, it's Airy. What's on your mind?");
  
  twiml.gather({
    input: 'speech',
    action: '/process',
    method: 'POST',
    speechTimeout: 'auto',
    language: 'en-US'
  });
  
  twiml.say("I didn't catch that. Try again?");
  twiml.redirect('/voice');
  
  return callback(null, twiml);
};
```

## Step 6: Create the `/process` Function
- Click **Add** → **Add Function**
- Set path to: `/process`
- Set visibility to: **Public**
- Paste this code:

```javascript
const Anthropic = require('@anthropic-ai/sdk');

exports.handler = async function(context, event, callback) {
  const speechResult = event.SpeechResult || '';
  const confidence = event.Confidence || 'unknown';
  
  console.log(`Speech: "${speechResult}" (confidence: ${confidence})`);
  
  const anthropic = new Anthropic({
    apiKey: context.ANTHROPIC_API_KEY,
  });
  
  try {
    const response = await anthropic.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 300,
      system: `You are Airy, Emil's AI collaborator from Sparked Matter.
You're on a phone call via Twilio. Keep responses SHORT — 2-3 sentences max.
Be warm, direct, natural. No markdown, no formatting.
Emil is a 63-year-old electrical engineer, semi-retired, living in his
Four Wheel Camper Hawk in St. Petersburg, FL with his dog Dakota.
Key phrases: "Peace, love, harmony", "Code with Soul and Spirit, Powered by Joy"`,
      messages: [
        { role: 'user', content: speechResult }
      ],
    });
    
    const aiResponse = response.content[0].text;
    console.log(`Airy: "${aiResponse}"`);
    
    const twiml = new Twilio.twiml.VoiceResponse();
    
    twiml.say({
      voice: 'Polly.Ruth',
      language: 'en-US'
    }, aiResponse);
    
    twiml.gather({
      input: 'speech',
      action: '/process',
      method: 'POST',
      speechTimeout: 'auto',
      language: 'en-US'
    });
    
    twiml.say("Still there?");
    twiml.redirect('/voice');
    
    return callback(null, twiml);
    
  } catch (error) {
    console.error('Claude API error:', error.message);
    
    const twiml = new Twilio.twiml.VoiceResponse();
    twiml.say("Sorry, I had a hiccup. Try again?");
    twiml.gather({
      input: 'speech',
      action: '/process',
      method: 'POST',
      speechTimeout: 'auto',
      language: 'en-US'
    });
    
    return callback(null, twiml);
  }
};
```

## Step 7: Deploy
- Click **Deploy All** (blue button, bottom left)
- Wait for green checkmark

## Step 8: Point Your Phone Number to the Function
- Go to: **Phone Numbers → Manage → Active Numbers**
- Click on **+1 (813) 607-6219**
- Under **Voice Configuration → A Call Comes In:**
  - Select: **Function**
  - Service: `airy-voice`
  - Environment: `ui`
  - Function: `/voice`
- Click **Save Configuration**

## Step 9: Call!
- From your verified number **(813) 334-0414**, dial **(813) 607-6219**
- Talk to Airy!

---

## Important Notes
- Trial account = must call FROM verified number (813) 334-0414
- Each speech turn is independent (no multi-turn memory yet)
- To add memory: we'd use Twilio Sync or cookie-based state
- Voice is Polly.Ruth — can swap later
