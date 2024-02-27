package org.archipel.generator;

public class Mth
{
    public static float sqrt(float pValue)
    {
        return (float)Math.sqrt(pValue);
    }

    public static int floor(float val)
    {
        final int i = (int)val;
        return val < (float)i ? i - 1 : i;
    }

    public static int clamp(int pValue, int pMin, int pMax)
    {
        return Math.min(Math.max(pValue, pMin), pMax);
    }

    public static int rgba(int alpha, int red, int green, int blue)
    {
        // 0xAARRGGBB
        //return (alpha << 24) | (red << 16) | (green << 8) | blue;
        return (clamp(alpha, 0, 255) << 24) | (clamp(red, 0, 255) << 16) | (clamp(green, 0, 255) << 8) | clamp(blue, 0, 255);
    }
}
