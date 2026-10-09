# secure stream の用語

[terms.md](../terms.md) の一部。secure connection に閉じる語の層ごとの対応を持つ。

<a id="t-omni-secure-auth"></a>
## OmniSecureAuth

**層ごとの名前**
constructor の `OmniSecureAuth::Anonymous` は、V2 profile の `AuthType::None`（tag 1）に対応する。
constructor の `OmniSecureAuth::Mutual { signer }` は、V2 profile の `AuthType::Sign`（tag 2）に対応する。
V2 は constructor で選んだ設定を双方に要求し、wire の相手申告から設定を選び直さない。
