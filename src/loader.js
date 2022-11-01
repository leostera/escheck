(async () => {
  const rule = await import("{MODULE_PATH}");
  EsCheck.Rule.new(rule.default);
})();
