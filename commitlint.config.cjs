module.exports = {
  extends: ['@commitlint/config-conventional'],
  ignores: [
    (message) => message.includes('Co-authored-by: codacy-production[bot]'),
    (message) => /^Update .*/.test(message) && message.includes('bot'),
  ],
};
