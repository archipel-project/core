package org.archipel.generator;

import java.util.Random;

public class Generator
{
    private static final float INPUT_FACTOR = 1.0181268882175227f;
    private final long seed;
    private final ImprovedPerlinNoise perlinNoise;
    private final Random random;

    public Generator(long seed)
    {
        this.seed = seed;
        this.perlinNoise = new ImprovedPerlinNoise(this.random = new Random(this.seed));
    }

    private static final int STONE_LEVEL = -39;
    private static final int SEA_LEVEL = -14;
    private static final int SNOW_LEVEL = 22;

    public int getBlock(int x, int y, int z)
    {
        final var noise = this.perlinNoise.fractalBrownianMotion(x * INPUT_FACTOR, z * INPUT_FACTOR, 8);
        final var surfaceLevel = Math.round(noise * 35);

        if(y >= SNOW_LEVEL && y <= surfaceLevel)
            return 11;

        if(y == surfaceLevel)
            return 3;

        if(y < surfaceLevel)
        {
            if(y < STONE_LEVEL)
                return 1;
            return 5;
        }

        if(y < SEA_LEVEL)
            return 4;

        return 0;
    }

    public long getSeed()
    {
        return this.seed;
    }
}
