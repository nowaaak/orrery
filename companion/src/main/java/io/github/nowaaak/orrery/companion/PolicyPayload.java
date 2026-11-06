package io.github.nowaaak.orrery.companion;

//? if >=1.20.5 {
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.resources.Identifier;

public record PolicyPayload(String json) implements CustomPacketPayload {
    public static final CustomPacketPayload.Type<PolicyPayload> TYPE =
            new CustomPacketPayload.Type<>(Identifier.fromNamespaceAndPath("orrery", "policy"));
    public static final StreamCodec<FriendlyByteBuf, PolicyPayload> CODEC = StreamCodec.of(
            (buf, payload) -> buf.writeUtf(payload.json(), Policy.MAX_BYTES),
            buf -> new PolicyPayload(buf.readUtf(Policy.MAX_BYTES)));

    @Override
    public CustomPacketPayload.Type<? extends CustomPacketPayload> type() {
        return TYPE;
    }
}
//?} else {
/*public final class PolicyPayload {
    private PolicyPayload() {
    }
}
*///?}
