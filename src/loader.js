(async () => {
  const rule = await import("{MODULE_PATH}");
  EsCheck.Rule.new({ name: "{RULE_NAME}", ...rule.default });
})();
